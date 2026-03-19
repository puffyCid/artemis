use super::{error::FormatError, timeline::timeline_data};
use crate::{
    artifacts::os::systeminfo::info::{get_info_metadata, hostname},
    structs::toml::Output,
    utils::{
        logging::collection_status,
        output::final_output,
        time::{time_now, unixepoch_to_iso},
        uuid::generate_uuid,
    },
};
use log::error;
use serde_json::{Value, json};

/// Output to `jsonl` files
pub(crate) fn jsonl_format(
    serde_data: &mut Value,
    artifact_name: &str,
    output: &mut Output,
    start_time: u64,
) -> Result<(), FormatError> {
    // Get small amount of system metadata
    let info = get_info_metadata();

    let uuid = generate_uuid();
    let filename = format!("{artifact_name}_{uuid}");

    let complete = unixepoch_to_iso(time_now() as i64);

    // If our data is an array loop through each element and output as a separate line
    if serde_data.is_array() {
        let mut empty_vec = Vec::new();
        // If we are timelining data. Timeline now before appending collection metadata
        if output.timeline {
            timeline_data(serde_data, artifact_name);
        }

        let entries = serde_data.as_array_mut().unwrap_or(&mut empty_vec);
        // If array is empty just output metadata
        if entries.is_empty() {
            let collection_output = json![{
                    "endpoint_id": output.endpoint_id,
                    "id": output.collection_id,
                    "uuid": uuid,
                    "artifact_name": artifact_name,
                    "complete_time": unixepoch_to_iso(time_now() as i64),
                    "start_time": unixepoch_to_iso(start_time as i64),
                    "hostname": info.hostname,
                    "os_version": info.os_version,
                    "platform": info.platform,
                    "kernel_version": info.kernel_version,
                    "load_performance": info.performance,
                    "version": info.version,
                    "rust_version": info.rust_version,
                    "build_date": info.build_date,
                    "interfaces": info.interfaces,
            }];
            let status = final_output(&collection_output, output, &filename);
            if let Err(result) = status {
                error!("[forensics] Failed to output {artifact_name} data: {result:?}");
            }
        } else {
            for entry in entries {
                if entry.is_object() {
                    entry["collection_metadata"] = json![{
                            "endpoint_id": output.endpoint_id,
                            "uuid": uuid,
                            "id": output.collection_id,
                            "artifact_name": artifact_name,
                            "complete_time": complete,
                            "start_time": unixepoch_to_iso(start_time as i64),
                            "hostname": info.hostname,
                            "os_version": info.os_version,
                            "platform": info.platform,
                            "kernel_version": info.kernel_version,
                            "load_performance": info.performance,
                            "version": info.version,
                            "rust_version": info.rust_version,
                            "build_date": info.build_date,
                            "interfaces": info.interfaces,
                    }];
                }
            }

            let status = final_output(serde_data, output, &filename);
            if let Err(result) = status {
                error!("[forensics] Failed to output {artifact_name} data: {result:?}");
            }
        }
    } else {
        if serde_data.is_object() {
            serde_data["collection_metadata"] = json![{
                    "endpoint_id": output.endpoint_id,
                    "uuid": uuid,
                    "id": output.collection_id,
                    "artifact_name": artifact_name,
                    "complete_time": complete,
                    "start_time": unixepoch_to_iso(start_time as i64),
                    "hostname": info.hostname,
                    "os_version": info.os_version,
                    "platform": info.platform,
                    "kernel_version": info.kernel_version,
                    "load_performance": info.performance
            }];
        }
        let status = final_output(serde_data, output, &filename);
        if let Err(result) = status {
            error!("[forensics] Failed to output {artifact_name} data: {result:?}");
        }
    }

    let _ = collection_status(&info.hostname, output, &filename);

    Ok(())
}

/// Output to `jsonl` files without metadata
pub(crate) fn raw_jsonl(
    serde_data: &Value,
    artifact_name: &str,
    output: &mut Output,
) -> Result<(), FormatError> {
    let uuid = generate_uuid();
    let filename = format!("{artifact_name}_{uuid}");
    // If our data is an array loop through each element and output as a separate line
    if serde_data.is_array() {
        let empty_vec = Vec::new();
        let entries = serde_data.as_array().unwrap_or(&empty_vec);
        if entries.is_empty() {
            return Ok(());
        }
        let status = final_output(serde_data, output, &filename);
        if let Err(result) = status {
            error!("[forensics] Failed to output {artifact_name} data: {result:?}");
        }
    } else {
        let status = final_output(serde_data, output, &filename);
        if let Err(result) = status {
            error!("[forensics] Failed to output {artifact_name} data: {result:?}");
        }
    }

    let _ = collection_status(&hostname(), output, &filename);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::raw_jsonl;
    use crate::{
        output::formats::jsonl::jsonl_format, structs::toml::Output, utils::time::time_now,
    };
    use serde_json::json;

    #[test]
    fn test_jsonl_format() {
        let mut output = Output {
            name: String::from("format_test"),
            directory: String::from("./tmp"),
            format: String::from("jsonl"),
            compress: false,
            endpoint_id: String::from("abcd"),
            output: String::from("local"),
            ..Default::default()
        };
        let start_time = time_now();

        let name = "test";
        let mut data = json!({"test":"test"});
        jsonl_format(&mut data, name, &mut output, start_time).unwrap();
    }

    #[test]
    fn test_raw_jsonl() {
        let mut output = Output {
            name: String::from("format_test"),
            directory: String::from("./tmp"),
            format: String::from("jsonl"),
            compress: false,
            endpoint_id: String::from("abcd"),
            output: String::from("local"),
            ..Default::default()
        };

        let name = "test";
        let data = serde_json::Value::String(String::from("test"));
        raw_jsonl(&data, name, &mut output).unwrap();
    }
}
