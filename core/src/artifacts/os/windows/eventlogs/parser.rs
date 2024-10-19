use std::collections::HashMap;

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
    strings::{get_resources, StringResource},
};
use crate::{
    artifacts::os::windows::artifacts::output_data,
    filesystem::files::{file_extension, list_files},
    structs::{artifacts::os::windows::EventLogsOptions, toml::Output},
    utils::{environment::get_systemdrive, regex_options::create_regex, time::time_now},
};
use chrono::SecondsFormat;
use common::windows::EventLogRecord;
use evtx::EvtxParser;
use log::error;

/// Parse `EventLogs` based on `EventLogsOptions`
pub(crate) fn grab_eventlogs(
    options: &EventLogsOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), EventLogsError> {
    if let Some(file) = &options.alt_file {
        return alt_eventlogs(file, output, filter, &options.include_template_strings);
    }

    default_eventlogs(output, filter, &options.include_template_strings)
}

/// Parse the `EventLog` evtx file at provided path
pub(crate) fn parse_eventlogs(path: &str) -> Result<Vec<EventLogRecord>, EventLogsError> {
    let evt_parser_results = EvtxParser::from_path(path);
    let mut evt_parser = match evt_parser_results {
        Ok(result) => result,
        Err(err) => {
            error!("[eventlogs] Failed to parse event log {path}, error: {err:?}");
            return Err(EventLogsError::Parser);
        }
    };

    let mut eventlog_records: Vec<EventLogRecord> = Vec::new();
    for record in evt_parser.records_json_value() {
        match record {
            Ok(data) => {
                let event_record = EventLogRecord {
                    event_record_id: data.event_record_id,
                    timestamp: data.timestamp.to_rfc3339_opts(SecondsFormat::Nanos, true),
                    data: data.data,
                };
                eventlog_records.push(event_record);
            }
            Err(err) => {
                error!("[eventlogs] Issue parsing record from {path}, error: {err:?}");
                continue;
            }
        }
    }
    Ok(eventlog_records)
}

/// Read and parse `EventLog` files at default Windows path. Typically C:\Windows\System32\winevt
fn default_eventlogs(
    output: &mut Output,
    filter: &bool,
    include_templates: &bool,
) -> Result<(), EventLogsError> {
    let drive_result = get_systemdrive();
    let drive = match drive_result {
        Ok(result) => result,
        Err(err) => {
            error!("[prefetch] Could not determine systemdrive: {err:?}");
            return Err(EventLogsError::DefaultDrive);
        }
    };
    let path = format!("{drive}:\\Windows\\System32\\winevt\\Logs");
    read_directory(&path, output, filter, include_templates)
}

/// Read and parse `EventLog` files with alternative path
fn alt_eventlogs(
    path: &str,
    output: &mut Output,
    filter: &bool,
    include_templates: &bool,
) -> Result<(), EventLogsError> {
    let mut templates = None;
    if *include_templates {
        templates = Some(get_resources()?);
    }
    read_eventlogs(path, output, filter, &templates)
}

/// Read all files at provided path
fn read_directory(
    path: &str,
    output: &mut Output,
    filter: &bool,
    include_templates: &bool,
) -> Result<(), EventLogsError> {
    let dir_results = list_files(path);
    let read_dir = match dir_results {
        Ok(result) => result,
        Err(err) => {
            error!("[eventlogs] Failed to get eventlogs files {path}, error: {err:?}");
            return Err(EventLogsError::Parser);
        }
    };
    let mut templates = None;
    if *include_templates {
        templates = Some(get_resources()?);
    }

    for evtx_file in read_dir {
        // Skip non-eventlog files
        if file_extension(&evtx_file) != "evtx" {
            continue;
        }

        let eventlogs_results = read_eventlogs(&evtx_file, output, filter, &templates);
        match eventlogs_results {
            Ok(_) => continue,
            Err(err) => {
                error!("[eventlogs] Failed to get eventlogs for {evtx_file}, error: {err:?}");
                continue;
            }
        }
    }

    Ok(())
}

/// Read and parse the `EventLog` file
fn read_eventlogs(
    path: &str,
    output: &mut Output,
    filter: &bool,
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
    let mut cache = HashMap::new();
    // Regex always correct
    let param_regex = create_regex(r"%\d+").unwrap();

    for record in evt_parser.records_json_value() {
        match record {
            Ok(data) => {
                let event_record = EventLogRecord {
                    event_record_id: data.event_record_id,
                    timestamp: data.timestamp.to_rfc3339_opts(SecondsFormat::Nanos, true),
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
            if let Some(resource) = resources {
                for record in &eventlog_records {
                    println!("{record:?}");
                    let message = add_message_strings(record, resource, &mut cache, &param_regex);
                    panic!("my message: {message:?}");
                }
            }

            let serde_data_result = serde_json::to_value(&eventlog_records);
            let serde_data = match serde_data_result {
                Ok(results) => results,
                Err(err) => {
                    error!("[eventlogs] Failed to serialize eventlogs: {err:?}");
                    return Err(EventLogsError::Serialize);
                }
            };

            let result = output_data(&serde_data, "eventlogs", output, &start_time, filter);
            match result {
                Ok(_result) => {}
                Err(err) => {
                    error!("[eventlogs] Could not output eventlogs data: {err:?}");
                }
            }

            eventlog_records = Vec::new();
        }
    }

    if !eventlog_records.is_empty() {
        if let Some(resource) = resources {
            for record in &eventlog_records {
                println!("{record:?}");
                let message =
                    add_message_strings(record, resource, &mut cache, &param_regex).unwrap();
                println!("my message: {message:?}");
            }
        }
        let serde_data_result = serde_json::to_value(&eventlog_records);
        let serde_data = match serde_data_result {
            Ok(results) => results,
            Err(err) => {
                error!("[eventlogs] Failed to serialize last eventlogs: {err:?}");
                return Err(EventLogsError::Serialize);
            }
        };

        let result = output_data(&serde_data, "eventlogs", output, &start_time, filter);
        match result {
            Ok(_result) => {}
            Err(err) => {
                error!("[eventlogs] Could not output last eventlogs data: {err:?}");
            }
        }
    }

    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{alt_eventlogs, default_eventlogs, grab_eventlogs, read_directory, read_eventlogs};
    use crate::{structs::artifacts::os::windows::EventLogsOptions, structs::toml::Output};
    use std::{fs::read_dir, path::PathBuf};

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_grab_eventlogs() {
        let options = EventLogsOptions {
            alt_file: None,
            include_template_strings: false,
        };
        let mut output = output_options("eventlog_temp", "local", "./tmp", true);

        let results = grab_eventlogs(&options, &mut output, &false).unwrap();
        assert_eq!(results, ())
    }

    #[test]
    fn test_default_eventlogs() {
        let mut output = output_options("eventlog_temp", "local", "./tmp", true);

        let results = default_eventlogs(&mut output, &false, &false).unwrap();
        assert_eq!(results, ())
    }

    #[test]
    #[should_panic(expected = "Parser")]
    fn test_alt_eventlogs() {
        let path = "madeup";
        let mut output = output_options("eventlog_temp", "local", "./tmp", true);

        let results = alt_eventlogs(&path, &mut output, &false, &false).unwrap();
        assert_eq!(results, ())
    }

    #[test]
    fn test_read_directory() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/eventlogs");
        let mut output = output_options("eventlog_temp", "local", "./tmp", false);

        let results = read_directory(
            &test_location.display().to_string(),
            &mut output,
            &false,
            &false,
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
                &false,
                &None,
            )
            .unwrap();
            assert_eq!(results, ())
        }
    }
}
