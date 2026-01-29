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
    artifacts::os::windows::artifacts::output_data,
    filesystem::files::{file_extension, list_files, read_file},
    output::formats::json::raw_json,
    structs::{artifacts::os::windows::EventLogsOptions, toml::Output},
    utils::{environment::get_systemdrive, regex_options::create_regex, time::time_now},
};
use common::windows::{EventLogRecord, EventMessage};
use evtx::EvtxParser;
use log::{error, warn};
use serde_json::{Error, Value};

/// Parse `EventLogs` based on `EventLogsOptions`
pub(crate) fn grab_eventlogs(
    options: &EventLogsOptions,
    output: &mut Output,
    filter: bool,
) -> Result<(), EventLogsError> {
    if let Some(file) = &options.alt_file {
        return alt_eventlogs(file, output, filter, options);
    }

    default_eventlogs(output, filter, options)
}

/// Parse the `EventLog` evtx file at provided path
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
        let bytes = match read_file(file) {
            Ok(result) => result,
            Err(err) => {
                error!("[eventlogs] Failed to read template file: {err:?}");
                return Err(EventLogsError::ReadTemplateFile);
            }
        };

        match serde_json::from_slice(&bytes) {
            Ok(result) => result,
            Err(err) => {
                error!("[eventlogs] Failed to deserialize template data: {err:?}");
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
            error!("[eventlogs] Failed to parse event log {path}, error: {err:?}");
            return Err(EventLogsError::Parser);
        }
    };

    let mut eventlog_records = Vec::new();
    // Regex always correct
    let param_regex = create_regex(r"(%\d!.*?!)|(%\d+)").unwrap();

    for record in evt_parser.records_json_value().skip(offset) {
        match record {
            Ok(data) => {
                let event_record = EventLogRecord {
                    event_record_id: data.event_record_id,
                    timestamp: data.timestamp.to_string(),
                    data: data.data,
                };
                eventlog_records.push(event_record);
            }
            Err(err) => {
                error!("[eventlogs] Issue parsing record from {path}, error: {err:?}");
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
            let mut message = if let Some(result) =
                add_message_strings(&record, resource, &param_regex)
            {
                result
            } else {
                warn!(
                    "[eventlogs] Could not get template strings for file {path} record: {}. Using raw data",
                    record.event_record_id
                );
                raw_messages.push(record);
                continue;
            };

            message.source_file = path.to_string();
            all_messages.push(message);
        }
        (all_messages, raw_messages)
    } else {
        (Vec::new(), eventlog_records)
    };

    Ok((messages, raw_message))
}

/// Read and parse `EventLog` files at default Windows path. Typically C:\Windows\System32\winevt
fn default_eventlogs(
    output: &mut Output,
    filter: bool,
    options: &EventLogsOptions,
) -> Result<(), EventLogsError> {
    let path = if options.alt_dir.is_some() {
        options.alt_dir.as_ref().unwrap()
    } else {
        let drive_result = get_systemdrive();
        let drive = match drive_result {
            Ok(result) => result,
            Err(err) => {
                error!("[eventlogs] Could not determine systemdrive: {err:?}");
                return Err(EventLogsError::DefaultDrive);
            }
        };
        &format!("{drive}:\\Windows\\System32\\winevt\\Logs")
    };

    read_directory(path, output, filter, options)
}

/// Read and parse `EventLog` files with alternative path
fn alt_eventlogs(
    path: &str,
    output: &mut Output,
    filter: bool,
    options: &EventLogsOptions,
) -> Result<(), EventLogsError> {
    let templates = if options.include_templates && options.alt_template_file.is_none() {
        Some(get_resources()?)
    } else if options.alt_template_file.is_some() {
        let template_file = options.alt_template_file.as_ref().unwrap();
        let bytes = match read_file(template_file) {
            Ok(result) => result,
            Err(err) => {
                error!("[eventlogs] Failed to read template file: {err:?}");
                return Err(EventLogsError::ReadTemplateFile);
            }
        };

        match serde_json::from_slice(&bytes) {
            Ok(result) => result,
            Err(err) => {
                error!("[eventlogs] Failed to deserialize template data: {err:?}");
                return Err(EventLogsError::DeserializeTemplate);
            }
        }
    } else {
        None
    };

    if templates.is_some() && options.dump_templates {
        output_logs(
            &mut serde_json::to_value(&templates),
            output,
            filter,
            0,
            "eventlog_templates",
            true,
        )?;

        if options.only_templates {
            return Ok(());
        }
    }

    read_eventlogs(path, output, filter, &templates)
}

/// Read all files at provided path
fn read_directory(
    path: &str,
    output: &mut Output,
    filter: bool,
    options: &EventLogsOptions,
) -> Result<(), EventLogsError> {
    let dir_results = list_files(path);
    let read_dir = match dir_results {
        Ok(result) => result,
        Err(err) => {
            error!("[eventlogs] Failed to get eventlogs files {path}, error: {err:?}");
            return Err(EventLogsError::Parser);
        }
    };

    let templates = if options.include_templates && options.alt_template_file.is_none() {
        Some(get_resources()?)
    } else if options.alt_template_file.is_some() {
        let template_file = options.alt_template_file.as_ref().unwrap();
        let bytes = match read_file(template_file) {
            Ok(result) => result,
            Err(err) => {
                error!("[eventlogs] Failed to read template file: {err:?}");
                return Err(EventLogsError::ReadTemplateFile);
            }
        };

        match serde_json::from_slice(&bytes) {
            Ok(result) => result,
            Err(err) => {
                error!("[eventlogs] Failed to deserialize template data: {err:?}");
                return Err(EventLogsError::DeserializeTemplate);
            }
        }
    } else {
        None
    };

    if templates.is_some() && options.dump_templates {
        output_logs(
            &mut serde_json::to_value(&templates),
            output,
            filter,
            0,
            "eventlog_templates",
            true,
        )?;

        if options.only_templates {
            return Ok(());
        }
    }

    for evtx_file in read_dir {
        // Skip non-eventlog files
        if file_extension(&evtx_file) != "evtx" {
            continue;
        }

        let eventlogs_results = read_eventlogs(&evtx_file, output, filter, &templates);
        match eventlogs_results {
            Ok(_) => (),
            Err(err) => {
                error!("[eventlogs] Failed to get eventlogs for {evtx_file}, error: {err:?}");
            }
        }
    }

    Ok(())
}

/// Read and parse the `EventLog` file
fn read_eventlogs(
    path: &str,
    output: &mut Output,
    filter: bool,
    resources: &Option<StringResource>,
) -> Result<(), EventLogsError> {
    let start_time = time_now();

    let evt_parser_results = EvtxParser::from_path(path);
    let mut evt_parser = match evt_parser_results {
        Ok(result) => result,
        Err(err) => {
            error!("[eventlogs] Failed to parse event log {path}, error: {err:?}");
            return Err(EventLogsError::Parser);
        }
    };

    let mut eventlog_records: Vec<EventLogRecord> = Vec::new();
    let limit = 10000;
    // Regex always correct
    let param_regex = create_regex(r"(%\d!.*?!)|(%\d+)").unwrap();

    for record in evt_parser.records_json_value() {
        match record {
            Ok(data) => {
                let event_record = EventLogRecord {
                    event_record_id: data.event_record_id,
                    timestamp: data.timestamp.to_string(),
                    data: data.data,
                };
                eventlog_records.push(event_record);
            }
            Err(err) => {
                error!("[eventlogs] Issue parsing record from {path}, error: {err:?}");
                continue;
            }
        }

        if eventlog_records.len() == limit {
            let (mut serde_data_result, raw_output) = if let Some(resource) = resources {
                let mut all_messages = Vec::new();
                let mut raw_messages = Vec::new();
                for record in eventlog_records {
                    let mut message = if let Some(result) =
                        add_message_strings(&record, resource, &param_regex)
                    {
                        result
                    } else {
                        warn!(
                            "[eventlogs] Could not get template strings for file {path} record: {}. Using raw data",
                            record.event_record_id
                        );
                        raw_messages.push(record);
                        continue;
                    };

                    message.source_file = path.to_string();
                    all_messages.push(message);
                }
                (serde_json::to_value(&all_messages), raw_messages)
            } else {
                (serde_json::to_value(&eventlog_records), Vec::new())
            };

            // If we failed to combine log data and strings. Then output the raw data
            if !raw_output.is_empty() {
                output_logs(
                    &mut serde_json::to_value(&raw_output),
                    output,
                    filter,
                    start_time,
                    "eventlogs",
                    false,
                )?;
            }

            output_logs(
                &mut serde_data_result,
                output,
                filter,
                start_time,
                "eventlogs",
                false,
            )?;

            eventlog_records = Vec::new();
        }
    }

    if !eventlog_records.is_empty() {
        let (mut serde_data_result, raw_output) = if let Some(resource) = resources {
            let mut all_messages = Vec::new();
            let mut raw_messages = Vec::new();
            for record in eventlog_records {
                let mut message = if let Some(result) =
                    add_message_strings(&record, resource, &param_regex)
                {
                    result
                } else {
                    warn!(
                        "[eventlogs] Could not get template strings for file {path} record: {}. Using raw data",
                        record.event_record_id
                    );
                    raw_messages.push(record);
                    continue;
                };
                message.source_file = path.to_string();
                all_messages.push(message);
            }
            (serde_json::to_value(&all_messages), raw_messages)
        } else {
            (serde_json::to_value(&eventlog_records), Vec::new())
        };

        // If we failed to combine log data and strings. Then output the raw data
        if !raw_output.is_empty() {
            output_logs(
                &mut serde_json::to_value(&raw_output),
                output,
                filter,
                start_time,
                "eventlogs",
                false,
            )?;
        }

        output_logs(
            &mut serde_data_result,
            output,
            filter,
            start_time,
            "eventlogs",
            false,
        )?;
    }

    Ok(())
}

/// Output log results
fn output_logs(
    result: &mut Result<Value, Error>,
    output: &mut Output,
    filter: bool,
    start_time: u64,
    name: &str,
    raw: bool,
) -> Result<(), EventLogsError> {
    let serde_data = match result {
        Ok(results) => results,
        Err(err) => {
            error!("[eventlogs] Failed to serialize last eventlogs: {err:?}");
            return Err(EventLogsError::Serialize);
        }
    };

    // Skip adding metadata to the output if we are just dumping templates
    if raw {
        let status = raw_json(serde_data, name, output);
        if let Err(result) = status {
            error!("[eventlogs] Could not output raw json results: {result:?}");
        }
        return Ok(());
    }

    match output_data(serde_data, name, output, start_time, filter) {
        Ok(_result) => {}
        Err(err) => {
            error!("[eventlogs] Could not output last eventlogs data: {err:?}");
        }
    }

    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use serde_json::json;

    use super::{
        alt_eventlogs, default_eventlogs, grab_eventlogs, output_logs, read_directory,
        read_eventlogs,
    };
    use crate::{structs::artifacts::os::windows::EventLogsOptions, structs::toml::Output};
    use std::{fs::read_dir, path::PathBuf};

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
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
        let mut output = output_options("eventlog_temp", "local", "./tmp", true);

        let results = grab_eventlogs(&options, &mut output, false).unwrap();
        assert_eq!(results, ())
    }

    #[test]
    fn test_default_eventlogs() {
        let mut output = output_options("eventlog_temp", "local", "./tmp", true);
        let options = EventLogsOptions {
            alt_file: None,
            include_templates: false,
            dump_templates: false,
            alt_dir: None,
            alt_template_file: None,
            only_templates: false,
        };

        let results = default_eventlogs(&mut output, false, &options).unwrap();
        assert_eq!(results, ())
    }

    #[test]
    #[should_panic(expected = "Parser")]
    fn test_alt_eventlogs() {
        let path = "madeup";
        let mut output = output_options("eventlog_temp", "local", "./tmp", true);

        let options = EventLogsOptions {
            alt_file: None,
            include_templates: false,
            dump_templates: false,
            alt_dir: None,
            alt_template_file: None,
            only_templates: false,
        };

        let results = alt_eventlogs(&path, &mut output, false, &options).unwrap();
        assert_eq!(results, ())
    }

    #[test]
    fn test_read_directory() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs");
        let mut output = output_options("eventlog_temp", "local", "./tmp", false);
        let options = EventLogsOptions {
            alt_file: None,
            include_templates: false,
            dump_templates: false,
            alt_dir: None,
            alt_template_file: None,
            only_templates: false,
        };

        let results = read_directory(
            &test_location.display().to_string(),
            &mut output,
            false,
            &options,
        )
        .unwrap();
        assert_eq!(results, ())
    }

    #[test]
    fn test_read_eventlogs() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs");
        let read_dir = read_dir(test_location.display().to_string()).unwrap();
        for file_path in read_dir {
            if file_path.as_ref().unwrap().file_type().unwrap().is_dir() {
                continue;
            }
            let mut output = output_options("eventlog_temp", "local", "./tmp", false);

            let results = read_eventlogs(
                &file_path.unwrap().path().display().to_string(),
                &mut output,
                false,
                &None,
            )
            .unwrap();
            assert_eq!(results, ())
        }
    }

    #[test]
    fn test_output_log() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs");
        let mut output = output_options("eventlog_temp", "local", "./tmp", false);

        let test = json!({"key": "value"});
        output_logs(&mut Ok(test), &mut output, false, 0, "testing", true).unwrap();
    }
}
