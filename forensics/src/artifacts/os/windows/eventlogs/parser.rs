/**
 * Windows `EventLogs` are the primary files associated with logging with system activity.
 * They are stored in a binary format, typically at C:\Windows\System32\winevt\Logs
 *
 * This parser uses the `evtx` crate to parse the `EventLogs`
 *  `https://github.com/omerbenamram/EVTX`
 *
 * Other Parsers:
 *  `https://github.com/Velocidex/velociraptor`
 *  Windows Event Viewer
 */
use super::{
    combine::add_message_strings,
    error::EventLogsError,
    strings::{StringResource, get_resources},
};
use crate::{
    accessor::{
        access::Accessor,
        entry::handle::{EntryKind, GlobMatch},
        io::reader::AccessorReader,
    },
    output::{
        manager::OutputManager,
        record::{
            SingleRecordStream, VecRecordStream, serialize_records_to_stream, serialize_to_record,
        },
    },
    structs::artifacts::os::windows::EventLogsOptions,
    utils::{environment::get_systemdrive, regex_options::create_regex},
};
use common::windows::{EventLogRecord, EventMessage};
use evtx::EvtxParser;
use tracing::{error, warn};

/// Parse `EventLogs` based on `EventLogsOptions`
pub(crate) fn grab_eventlogs(
    options: &EventLogsOptions,
    manager: &mut OutputManager,
) -> Result<(), EventLogsError> {
    let pattern = if let Some(file) = &options.alt_file {
        file.clone()
    } else {
        let drive_result = get_systemdrive();
        let drive = match drive_result {
            Ok(result) => result,
            Err(err) => {
                error!("Could not determine systemdrive: {err:?}");
                return Err(EventLogsError::DefaultDrive);
            }
        };
        format!("{drive}:\\Windows\\System32\\winevt\\Logs\\*.evtx")
    };

    let mut accessor = Accessor::with_defaults();
    let paths = match accessor.globfs(&pattern) {
        Ok(results) => results,
        Err(err) => {
            error!("Could not glob Eventlogs {pattern}: {err:?}");
            return Err(EventLogsError::Parser);
        }
    };

    extract_eventlogs(paths, options, manager)
}

/// Parse the `EventLog` evtx file at provided path
///
/// Used by `BoaJS` to reading `EventLogs` in chunk
pub(crate) fn parse_eventlogs(
    path: &str,
    offset: usize,
    limit: usize,
    include_templates: bool,
    template_file: &Option<String>,
) -> Result<(Vec<EventMessage>, Vec<EventLogRecord>), EventLogsError> {
    let templates = if include_templates && template_file.is_none() {
        Some(get_resources()?)
    } else if template_file.is_some() {
        let file = template_file.as_ref().unwrap();
        let bytes = match Accessor::with_defaults().read_file(file) {
            Ok(result) => result,
            Err(err) => {
                error!("Failed to read template file: {err:?}");
                return Err(EventLogsError::ReadTemplateFile);
            }
        };

        match serde_json::from_slice(&bytes) {
            Ok(result) => result,
            Err(err) => {
                error!("Failed to deserialize template data: {err:?}");
                return Err(EventLogsError::DeserializeTemplate);
            }
        }
    } else {
        None
    };

    let evt_parser_results = EvtxParser::from_path(path);
    let mut evt_parser = match evt_parser_results {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to parse event log {path}, error: {err:?}");
            return Err(EventLogsError::Parser);
        }
    };

    let mut eventlog_records = Vec::new();
    // Regex always correct
    let param_regex = create_regex(r"(%\d!.*?!)|(%\d+)").unwrap();
    let value_regex = create_regex(r"%%\d+").unwrap();

    for record in evt_parser.records_json_value().skip(offset) {
        match record {
            Ok(data) => {
                let event_record = EventLogRecord {
                    event_record_id: data.event_record_id,
                    timestamp: data.timestamp.to_string(),
                    data: data.data,
                    evidence: path.to_string(),
                };
                eventlog_records.push(event_record);
            }
            Err(err) => {
                error!("Issue parsing record from {path}, error: {err:?}");
                continue;
            }
        }

        if eventlog_records.len() == limit {
            break;
        }
    }
    let (messages, raw_message) = if let Some(resource) = &templates {
        let mut all_messages = Vec::new();
        let mut raw_messages = Vec::new();
        for record in eventlog_records {
            let message = if let Some(result) =
                add_message_strings(&record, resource, &param_regex, &value_regex, path)
            {
                result
            } else {
                warn!(
                    "Could not get template strings for file {path} record: {}. Using raw data",
                    record.event_record_id
                );
                raw_messages.push(record);
                continue;
            };

            all_messages.push(message);
        }
        (all_messages, raw_messages)
    } else {
        (Vec::new(), eventlog_records)
    };

    Ok((messages, raw_message))
}

/// Stream and extract `EventLog` entries
fn extract_eventlogs(
    paths: Vec<GlobMatch>,
    options: &EventLogsOptions,
    manager: &mut OutputManager,
) -> Result<(), EventLogsError> {
    let templates = if options.include_templates && options.alt_template_file.is_none() {
        Some(get_resources()?)
    } else if options.alt_template_file.is_some() {
        let template_file = options.alt_template_file.as_ref().unwrap();
        let bytes = match Accessor::with_defaults().read_file(template_file) {
            Ok(result) => result,
            Err(err) => {
                error!("Failed to read template file: {err:?}");
                return Err(EventLogsError::ReadTemplateFile);
            }
        };

        match serde_json::from_slice(&bytes) {
            Ok(result) => result,
            Err(err) => {
                error!("Failed to deserialize template data: {err:?}");
                return Err(EventLogsError::DeserializeTemplate);
            }
        }
    } else {
        None
    };

    if let Some(template) = templates.as_ref()
        && options.dump_templates
    {
        let record = match serialize_to_record(template) {
            Ok(result) => result,
            Err(err) => {
                error!("Failed to serialize provider strings: {err:?}");
                return Err(EventLogsError::Serialize);
            }
        };
        let artifact_name = "eventlog_templates";
        if let Err(err) =
            manager.write_artifact(artifact_name, options, &mut SingleRecordStream::new(record))
        {
            error!("Failed to output provider strings: {err:?}");
            return Err(EventLogsError::Output);
        }

        if options.only_templates {
            return Ok(());
        }
    }
    let mut accessor = Accessor::with_defaults();
    for entry in paths {
        if entry.meta.kind != EntryKind::File {
            continue;
        }

        let Some(handle) = entry.handle.as_file() else {
            continue;
        };

        if !handle.extension().eq_ignore_ascii_case("evtx") {
            continue;
        }

        let mut evtx_reader = match accessor.open_reader_handle(handle) {
            Ok(results) => results,
            Err(err) => {
                error!(
                    "Could not create reader for {}: {err:?}",
                    handle.display_path()
                );
                continue;
            }
        };

        read_eventlogs(
            &mut evtx_reader,
            manager,
            options,
            &templates,
            &handle.display_path(),
        )?;
    }

    Ok(())
}

/// Read and parse the `EventLog` file
fn read_eventlogs(
    reader: &mut AccessorReader,
    manager: &mut OutputManager,
    options: &EventLogsOptions,
    resources: &Option<StringResource>,
    evidence: &str,
) -> Result<(), EventLogsError> {
    let evt_parser_results = EvtxParser::from_read_seek(reader);
    let mut evt_parser = match evt_parser_results {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to parse event log {evidence}, error: {err:?}");
            return Err(EventLogsError::Parser);
        }
    };

    let mut eventlog_records = Vec::new();
    let limit = 1000;
    // Regex always correct
    let param_regex = create_regex(r"(%\d!.*?!)|(%\d+)").unwrap();
    let value_regex = create_regex(r"%%\d+").unwrap();
    for record in evt_parser.records_json_value() {
        match record {
            Ok(data) => {
                let event_record = EventLogRecord {
                    event_record_id: data.event_record_id,
                    timestamp: data.timestamp.to_string(),
                    data: data.data,
                    evidence: evidence.to_string(),
                };
                eventlog_records.push(event_record);
            }
            Err(err) => {
                error!("Issue parsing record from {evidence}, error: {err:?}");
                continue;
            }
        }

        if eventlog_records.len() == limit {
            let (messages, raw_output) = if let Some(resource) = resources {
                let mut all_messages = Vec::new();
                let mut raw_messages = Vec::new();
                for record in eventlog_records {
                    let message = if let Some(result) =
                        add_message_strings(&record, resource, &param_regex, &value_regex, evidence)
                    {
                        result
                    } else {
                        warn!(
                            "Could not get template strings for file {evidence} record: {}. Using raw data",
                            record.event_record_id
                        );
                        raw_messages.push(record);
                        continue;
                    };

                    all_messages.push(message);
                }

                (all_messages, raw_messages)
            } else {
                (Vec::new(), eventlog_records)
            };

            // If we failed to combine log data and strings. Then output any log raw data
            if !raw_output.is_empty() {
                let _ = match serialize_records_to_stream(raw_output) {
                    Ok(records) => output_logs(manager, options, records),
                    Err(err) => {
                        error!("Could not serialize raw logs: {err:?}");
                        return Err(EventLogsError::Serialize);
                    }
                };
            }

            if !messages.is_empty() {
                let records = match serialize_records_to_stream(messages) {
                    Ok(result) => result,
                    Err(err) => {
                        error!("Could not serialize logs: {err:?}");
                        eventlog_records = Vec::new();
                        continue;
                    }
                };
                let _ = output_logs(manager, options, records);
            }

            eventlog_records = Vec::new();
        }
    }

    if !eventlog_records.is_empty() {
        let (messages, raw_output) = if let Some(resource) = resources {
            let mut all_messages = Vec::new();
            let mut raw_messages = Vec::new();
            for record in eventlog_records {
                let message = if let Some(result) =
                    add_message_strings(&record, resource, &param_regex, &value_regex, evidence)
                {
                    result
                } else {
                    warn!(
                        "Could not get template strings for file {evidence} record: {}. Using raw data",
                        record.event_record_id
                    );
                    raw_messages.push(record);
                    continue;
                };
                all_messages.push(message);
            }
            (all_messages, raw_messages)
        } else {
            (Vec::new(), eventlog_records)
        };

        // If we failed to combine log data and strings. Then output any log raw data
        if !raw_output.is_empty() {
            let _ = match serialize_records_to_stream(raw_output) {
                Ok(records) => output_logs(manager, options, records),
                Err(err) => {
                    error!("Could not serialize remaining raw logs: {err:?}");
                    return Err(EventLogsError::Serialize);
                }
            };
        }

        if !messages.is_empty() {
            let records = match serialize_records_to_stream(messages) {
                Ok(result) => result,
                Err(err) => {
                    error!("Could not serialize remaining logs: {err:?}");
                    return Err(EventLogsError::Serialize);
                }
            };
            let _ = output_logs(manager, options, records);
        }
    }

    Ok(())
}

/// Output log results
fn output_logs(
    manager: &mut OutputManager,
    options: &EventLogsOptions,
    mut records: VecRecordStream,
) -> Result<(), EventLogsError> {
    let artifact_name = "eventlogs";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("Failed to output eventlogs: {err:?}");
        return Err(EventLogsError::Output);
    }

    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{grab_eventlogs, read_eventlogs};
    use crate::accessor::access::Accessor;
    use crate::artifacts::os::windows::eventlogs::parser::extract_eventlogs;
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        output::manager::OutputManager, structs::artifacts::os::windows::EventLogsOptions,
    };
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_grab_eventlogs() {
        let options = EventLogsOptions {
            alt_file: None,
            include_templates: false,
            dump_templates: false,
            alt_dir: None,
            alt_template_file: None,
            only_templates: false,
        };
        let mut output = output_options("eventlog_temp", "./tmp", true);

        grab_eventlogs(&options, &mut output).unwrap();
    }

    #[test]
    fn test_extract_eventlogs() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs");
        let mut accessor = Accessor::with_defaults();
        let paths = accessor.globfs(test_location.to_str().unwrap()).unwrap();
        let options = EventLogsOptions {
            alt_file: None,
            include_templates: false,
            dump_templates: false,
            alt_dir: None,
            alt_template_file: None,
            only_templates: false,
        };
        let mut output = output_options("eventlog_temp", "./tmp", true);

        extract_eventlogs(paths, &options, &mut output).unwrap();
    }

    #[test]
    fn test_read_eventlogs() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs");
        let mut accessor = Accessor::with_defaults();
        let files = accessor.read_dir(test_location.to_str().unwrap()).unwrap();
        let options = EventLogsOptions {
            alt_file: None,
            include_templates: false,
            dump_templates: false,
            alt_dir: None,
            alt_template_file: None,
            only_templates: false,
        };
        for file_path in files {
            let Some(handle) = file_path.handle.as_file() else {
                continue;
            };

            let mut output = output_options("eventlog_temp", "./tmp", false);
            let mut reader = accessor.open_reader_handle(handle).unwrap();

            read_eventlogs(
                &mut reader,
                &mut output,
                &options,
                &None,
                &handle.display_path(),
            )
            .unwrap();
        }
    }
}
