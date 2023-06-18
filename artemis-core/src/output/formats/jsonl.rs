use super::error::FormatError;
use crate::{
    artifacts::os::systeminfo::info::SystemInfo,
    utils::{
        artemis_toml::Output, compression::compress_gzip, logging::collection_status,
        output::output_artifact, time::time_now, uuid::generate_uuid,
    },
};
use log::{error, info};
use serde_json::{json, Value};
use std::fs::remove_file;

/// Output to `jsonl` files
pub(crate) fn jsonl_format(
    serde_data: &Value,
    output_name: &str,
    output: &mut Output,
    start_time: &u64,
) -> Result<(), FormatError> {
    // Get small amount of system metadata
    let info = SystemInfo::get_info_metadata();
    let mut collection_output = json![{
        "metadata": {
            "endpoint_id": output.endpoint_id,
            "id": output.collection_id,
            "artifact_name": output_name,
            "complete_time": time_now(),
            "start_time": start_time,
            "hostname": info.hostname,
            "os_version": info.os_version,
            "platform": info.platform,
            "kernel_version": info.kernel_version,
            "load_performance": info.performance
        }
    }];

    let uuid = generate_uuid();
    // If our data is an array loop through each element and output as a separate line
    if serde_data.is_array() {
        let empty_vec = Vec::new();
        let entries = serde_data.as_array().unwrap_or(&empty_vec);
        // If array is empty just output metadata
        if entries.is_empty() {
            write_meta_json(&mut collection_output, output, output_name, &uuid)?;
        } else {
            let mut json_lines = Vec::new();
            for entry in entries {
                if output.output != "local" {
                    let line = create_line(&mut collection_output, entry)?;
                    json_lines.push(line);
                    continue;
                }
                let write_result =
                    write_full_json(&mut collection_output, entry, output, output_name, &uuid);
                match write_result {
                    Ok(_) => {}
                    Err(err) => {
                        error!(
                            "[artemis-core] Could not write jsonl file for {output_name}: {err:?}"
                        );
                    }
                }
            }
            if output.output != "local" {
                let output_result = output_artifact(json_lines.join("").as_bytes(), output, &uuid);
                match output_result {
                    Ok(_) => info!("[artemis-core] {output_name} jsonl output success"),
                    Err(err) => {
                        error!("[artemis-core] Failed to output {output_name} jsonl: {err:?}");
                        return Err(FormatError::Output);
                    }
                }
                let _ = collection_status(output_name, output, &uuid);

                return Ok(());
            }
        }
    } else {
        write_full_json(
            &mut collection_output,
            serde_data,
            output,
            output_name,
            &uuid,
        )?;
    }

    if output.compress && output.output == "local" {
        let path = format!("{}/{}/{}.jsonl", output.directory, output.name, uuid);
        let compress_result = compress_gzip(&path);
        match compress_result {
            Ok(_) => {
                let status = remove_file(&path);
                match status {
                    Ok(_) => {}
                    Err(err) => {
                        error!("[artemis-core] Could not remove old file at {path}: {err:?}");
                        return Err(FormatError::RemoveOldFile);
                    }
                }
            }
            Err(_err) => return Err(FormatError::Output),
        }
    }
    let _ = collection_status(output_name, output, &uuid);

    Ok(())
}

/// Write only the metadata JSON to the file
fn write_meta_json(
    base_data: &mut Value,
    output: &Output,
    output_name: &str,
    uuid: &str,
) -> Result<(), FormatError> {
    base_data["metadata"]["uuid"] = Value::String(generate_uuid());
    write_line(base_data, output, output_name, uuid)
}

/// Write the full expected JSON to file
fn write_full_json(
    base_data: &mut Value,
    artifact_data: &Value,
    output: &Output,
    output_name: &str,
    uuid: &str,
) -> Result<(), FormatError> {
    base_data["data"] = artifact_data.clone();
    base_data["metadata"]["uuid"] = Value::String(generate_uuid());
    write_line(base_data, output, output_name, uuid)
}

/// Create the a single JSON line
fn create_line(base_data: &mut Value, artifact_data: &Value) -> Result<String, FormatError> {
    base_data["data"] = artifact_data.clone();
    base_data["metadata"]["uuid"] = Value::String(generate_uuid());
    let serde_collection_results = serde_json::to_string(base_data);
    let serde_collection = match serde_collection_results {
        Ok(results) => format!("{results}\n"),
        Err(err) => {
            error!("[artemis-core] Failed to serialize jsonl output: {err:?}");
            return Err(FormatError::Serialize);
        }
    };
    Ok(serde_collection)
}

/// Write JSON line to file
fn write_line(
    base_data: &mut Value,
    output: &Output,
    output_name: &str,
    uuid: &str,
) -> Result<(), FormatError> {
    let serde_collection_results = serde_json::to_string(base_data);
    let serde_collection = match serde_collection_results {
        Ok(results) => format!("{results}\n"),
        Err(err) => {
            error!("[artemis-core] Failed to serialize jsonl output: {err:?}");
            return Err(FormatError::Serialize);
        }
    };

    let output_result = output_artifact(serde_collection.as_bytes(), output, uuid);
    match output_result {
        Ok(_) => info!("[artemis-core] {output_name} jsonl output success"),
        Err(err) => {
            error!("[artemis-core] Failed to output {output_name} jsonl: {err:?}");
            return Err(FormatError::Output);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{create_line, write_full_json, write_line, write_meta_json};
    use crate::{
        output::formats::jsonl::jsonl_format,
        utils::{artemis_toml::Output, time::time_now, uuid::generate_uuid},
    };
    use serde_json::json;

    #[test]
    fn test_output_data() {
        let mut output = Output {
            name: String::from("format_test"),
            directory: String::from("./tmp"),
            format: String::from("jsonl"),
            compress: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("json"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
        };
        let start_time = time_now();

        let name = "test";
        let data = serde_json::Value::String(String::from("test"));
        jsonl_format(&data, name, &mut output, &start_time).unwrap();
    }

    #[test]
    fn test_write_full_json() {
        let mut collection_output = json![{
            "endpoint_id": "test",
            "id": "1",
            "artifact_name": "test",
            "complete_time": time_now(),
            "start_time": 1,
        }];
        let output = Output {
            name: String::from("format_test"),
            directory: String::from("./tmp"),
            format: String::from("jsonl"),
            compress: false,
            url: Some(String::new()),

            api_key: Some(String::new()),

            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("json"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
        };

        let uuid = generate_uuid();
        write_full_json(
            &mut collection_output,
            &serde_json::Value::String(String::from("test")),
            &output,
            "test",
            &uuid,
        )
        .unwrap();
    }

    #[test]
    fn test_write_meta_json() {
        let mut collection_output = json![{
            "endpoint_id": "test",
            "id": "1",
            "artifact_name": "test",
            "complete_time": time_now(),
            "start_time": 1,
        }];
        let output = Output {
            name: String::from("format_test"),
            directory: String::from("./tmp"),
            format: String::from("jsonl"),
            compress: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("json"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
        };

        let uuid = generate_uuid();
        write_meta_json(&mut collection_output, &output, "test", &uuid).unwrap();
    }

    #[test]
    fn test_create_line() {
        let mut collection_output = json![{
            "endpoint_id": "test",
            "id": "1",
            "artifact_name": "test",
            "complete_time": time_now(),
            "start_time": 1,
        }];

        let mut data = serde_json::Value::String(String::from("test"));

        let line = create_line(&mut collection_output, &mut data).unwrap();
        assert!(!line.is_empty());
    }

    #[test]
    fn test_write_line() {
        let mut output = Output {
            name: String::from("format_test"),
            directory: String::from("./tmp"),
            format: String::from("jsonl"),
            compress: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("json"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
        };

        let name = "test";
        let mut data = serde_json::Value::String(String::from("test"));
        let uuid = generate_uuid();

        write_line(&mut data, &mut output, &name, &uuid).unwrap();
    }
}
