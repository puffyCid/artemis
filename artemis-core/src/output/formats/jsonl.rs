use super::error::FormatError;
use crate::{
    artifacts::os::systeminfo::info::get_info_metadata,
    structs::toml::Output,
    utils::{
        compression::compress_gzip_data, logging::collection_status, output::output_artifact,
        time::time_now, uuid::generate_uuid,
    },
};
use log::{error, info};
use serde_json::{json, Value};

/// Output to `jsonl` files
pub(crate) fn jsonl_format(
    serde_data: &Value,
    output_name: &str,
    output: &mut Output,
    start_time: &u64,
) -> Result<(), FormatError> {
    // Get small amount of system metadata
    let info = get_info_metadata();
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
            write_meta_json(&mut collection_output, output, &uuid)?;
        } else {
            let mut json_lines = Vec::new();
            for entry in entries {
                let line = create_line(&mut collection_output, entry)?;
                json_lines.push(line);
            }

            let collection_data = json_lines.join("");
            let status = write_json(collection_data.as_bytes(), output, &uuid);
            if status.is_err() {
                error!(
                    "[artemis-core] Failed to output {output_name} data: {:?}",
                    status.unwrap_err()
                );
            }
        }
    } else {
        let json_data = create_line(&mut collection_output, serde_data)?;
        let status = write_json(json_data.as_bytes(), output, &uuid);

        if status.is_err() {
            error!(
                "[artemis-core] Failed to output {output_name} data: {:?}",
                status.unwrap_err()
            );
        }
    }

    let _ = collection_status(output_name, output, &uuid);

    Ok(())
}

/// Write only the metadata JSON to the file
fn write_meta_json(
    base_data: &mut Value,
    output: &mut Output,
    uuid: &str,
) -> Result<(), FormatError> {
    base_data["metadata"]["uuid"] = Value::String(generate_uuid());
    let metadata = serde_json::to_vec(base_data).unwrap_or_default();
    write_json(&metadata, output, uuid)
}

/// Write JSONL bytes to file
fn write_json(data: &[u8], output: &mut Output, output_name: &str) -> Result<(), FormatError> {
    let output_data = if output.compress {
        let compressed_results = compress_gzip_data(data);
        match compressed_results {
            Ok(result) => result,
            Err(err) => {
                error!("[artemis-core] Failed to compress data: {err:?}");
                return Err(FormatError::Output);
            }
        }
    } else {
        data.to_vec()
    };

    let output_result = output_artifact(&output_data, output, output_name);
    match output_result {
        Ok(_) => info!("[artemis-core] {output_name} jsonl output success"),
        Err(err) => {
            error!("[artemis-core] Failed to output {output_name} jsonl: {err:?}");
            return Err(FormatError::Output);
        }
    }

    Ok(())
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

#[cfg(test)]
mod tests {
    use super::{create_line, write_json, write_meta_json};
    use crate::{
        output::formats::jsonl::jsonl_format,
        structs::toml::Output,
        utils::{time::time_now, uuid::generate_uuid},
    };
    use serde_json::json;

    #[test]
    fn test_jsonl_format() {
        let mut output = Output {
            name: String::from("format_test"),
            directory: String::from("./tmp"),
            format: String::from("jsonl"),
            compress: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        };
        let start_time = time_now();

        let name = "test";
        let data = serde_json::Value::String(String::from("test"));
        jsonl_format(&data, name, &mut output, &start_time).unwrap();
    }

    #[test]
    fn test_write_json() {
        let mut collection_output = json![{
            "metadata":{
                "endpoint_id": "test",
                "id": "1",
                "artifact_name": "test",
                "complete_time": time_now(),
                "start_time": 1,
            }
        }];

        let mut output = Output {
            name: String::from("format_test"),
            directory: String::from("./tmp"),
            format: String::from("jsonl"),
            compress: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        };

        let uuid = generate_uuid();
        let json_line = create_line(
            &mut collection_output,
            &serde_json::Value::String(String::from("test")),
        )
        .unwrap();
        write_json(json_line.as_bytes(), &mut output, &uuid).unwrap();
    }

    #[test]
    fn test_write_meta_json() {
        let mut collection_output = json![{
            "metadata":{
                "endpoint_id": "test",
                "id": "1",
                "artifact_name": "test",
                "complete_time": time_now(),
                "start_time": 1,
            }
        }];
        let mut output = Output {
            name: String::from("format_test"),
            directory: String::from("./tmp"),
            format: String::from("jsonl"),
            compress: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        };

        let uuid = generate_uuid();
        write_meta_json(&mut collection_output, &mut output, &uuid).unwrap();
    }

    #[test]
    fn test_create_line() {
        let mut collection_output = json![{
            "metadata":{
                "endpoint_id": "test",
                "id": "1",
                "artifact_name": "test",
                "complete_time": time_now(),
                "start_time": 1,
            }
        }];

        let mut data = serde_json::Value::String(String::from("test"));

        let line = create_line(&mut collection_output, &mut data).unwrap();
        assert!(!line.is_empty());
    }
}
