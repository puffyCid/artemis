use super::{error::FormatError, timeline::timeline_data};
use crate::{
    artifacts::os::systeminfo::info::get_info_metadata,
    structs::toml::Output,
    utils::{
        compression::compress::compress_gzip_bytes,
        logging::collection_status,
        output::final_output,
        time::{time_now, unixepoch_to_iso},
        uuid::generate_uuid,
    },
};
use log::{error, info};
use serde_json::{Value, json};

/// Output to `jsonl` files
pub(crate) fn jsonl_format(
    serde_data: &mut Value,
    output_name: &str,
    output: &mut Output,
    start_time: &u64,
) -> Result<(), FormatError> {
    // Get small amount of system metadata
    let info = get_info_metadata();

    let uuid = generate_uuid();
    let complete = unixepoch_to_iso(&(time_now() as i64));

    // If our data is an array loop through each element and output as a separate line
    if serde_data.is_array() {
        let mut empty_vec = Vec::new();
        // If we are timelining data. Timeline now before appending collection metadata
        if output.timeline {
            timeline_data(serde_data, output_name);
        }

        let entries = serde_data.as_array_mut().unwrap_or(&mut empty_vec);
        // If array is empty just output metadata
        if entries.is_empty() {
            let collection_output = json![{
                    "endpoint_id": output.endpoint_id,
                    "id": output.collection_id,
                    "uuid": uuid,
                    "artifact_name": output_name,
                    "complete_time": unixepoch_to_iso(&(time_now() as i64)),
                    "start_time": unixepoch_to_iso(&(*start_time as i64)),
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
            write_json(
                &serde_json::to_vec(&collection_output).unwrap_or_default(),
                output,
                &uuid,
            )?;
        } else {
            let mut json_lines = Vec::new();
            for entry in entries {
                if entry.is_object() {
                    entry["collection_metadata"] = json![{
                            "endpoint_id": output.endpoint_id,
                            "uuid": uuid,
                            "id": output.collection_id,
                            "artifact_name": output_name,
                            "complete_time": complete,
                            "start_time": unixepoch_to_iso(&(*start_time as i64)),
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

                let line = create_line(entry)?;
                json_lines.push(line);
            }

            let collection_data = json_lines.join("");
            let status = write_json(collection_data.as_bytes(), output, &uuid);
            if status.is_err() {
                error!(
                    "[core] Failed to output {output_name} data: {:?}",
                    status.unwrap_err()
                );
            }
        }
    } else {
        if serde_data.is_object() {
            serde_data["collection_metadata"] = json![{
                    "endpoint_id": output.endpoint_id,
                    "uuid": uuid,
                    "id": output.collection_id,
                    "artifact_name": output_name,
                    "complete_time": complete,
                    "start_time": unixepoch_to_iso(&(*start_time as i64)),
                    "hostname": info.hostname,
                    "os_version": info.os_version,
                    "platform": info.platform,
                    "kernel_version": info.kernel_version,
                    "load_performance": info.performance
            }];
        }

        let status = write_json(
            &serde_json::to_vec(serde_data).unwrap_or_default(),
            output,
            &uuid,
        );

        if status.is_err() {
            error!(
                "[core] Failed to output {output_name} data: {:?}",
                status.unwrap_err()
            );
        }
    }

    let _ = collection_status(output_name, output, &uuid);

    Ok(())
}

/// Output to `jsonl` files without metadata
pub(crate) fn raw_jsonl(
    serde_data: &Value,
    output_name: &str,
    output: &mut Output,
) -> Result<(), FormatError> {
    let uuid = generate_uuid();
    // If our data is an array loop through each element and output as a separate line
    if serde_data.is_array() {
        let empty_vec = Vec::new();
        let entries = serde_data.as_array().unwrap_or(&empty_vec);
        if entries.is_empty() {
            return Ok(());
        }

        let mut json_lines = Vec::new();
        for entry in entries {
            let line = create_line(entry)?;
            json_lines.push(line);
        }

        let collection_data = json_lines.join("");
        let status = write_json(collection_data.as_bytes(), output, &uuid);
        if status.is_err() {
            error!(
                "[core] Failed to output {output_name} raw data: {:?}",
                status.unwrap_err()
            );
        }
    } else {
        let status = write_json(
            &serde_json::to_vec(serde_data).unwrap_or_default(),
            output,
            &uuid,
        );

        if status.is_err() {
            error!(
                "[core] Failed to output {output_name} raw data: {:?}",
                status.unwrap_err()
            );
        }
    }

    let _ = collection_status(output_name, output, &uuid);

    Ok(())
}

/// Write JSONL bytes to file
fn write_json(data: &[u8], output: &mut Output, output_name: &str) -> Result<(), FormatError> {
    if output.compress {
        let compressed_results = compress_gzip_bytes(data);
        let compressed_data = match compressed_results {
            Ok(result) => result,
            Err(err) => {
                error!("[core] Failed to compress data: {err:?}");
                return Err(FormatError::Output);
            }
        };

        let output_result = final_output(&compressed_data, output, output_name);
        match output_result {
            Ok(_) => info!("[core] {output_name} jsonl output success"),
            Err(err) => {
                error!("[core] Failed to output {output_name} jsonl: {err:?}");
                return Err(FormatError::Output);
            }
        }

        return Ok(());
    }

    let output_result = final_output(data, output, output_name);
    match output_result {
        Ok(_) => info!("[core] {output_name} jsonl output success"),
        Err(err) => {
            error!("[core] Failed to output {output_name} jsonl: {err:?}");
            return Err(FormatError::Output);
        }
    }

    Ok(())
}

/// Create the a single JSON line
fn create_line(artifact_data: &Value) -> Result<String, FormatError> {
    let serde_collection_results = serde_json::to_string(artifact_data);
    let serde_collection = match serde_collection_results {
        Ok(results) => format!("{results}\n"),
        Err(err) => {
            error!("[core] Failed to serialize jsonl output: {err:?}");
            return Err(FormatError::Serialize);
        }
    };
    Ok(serde_collection)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{create_line, raw_jsonl, write_json};
    use crate::{
        output::formats::jsonl::jsonl_format,
        structs::toml::Output,
        utils::{time::time_now, uuid::generate_uuid},
    };

    #[test]
    fn test_jsonl_format() {
        let mut output = Output {
            name: String::from("format_test"),
            directory: String::from("./tmp"),
            format: String::from("jsonl"),
            compress: false,
            timeline: false,
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
        let mut data = json!({"test":"test"});
        jsonl_format(&mut data, name, &mut output, &start_time).unwrap();
    }

    #[test]
    fn test_raw_jsonl() {
        let mut output = Output {
            name: String::from("format_test"),
            directory: String::from("./tmp"),
            format: String::from("jsonl"),
            compress: false,
            timeline: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        };

        let name = "test";
        let data = serde_json::Value::String(String::from("test"));
        raw_jsonl(&data, name, &mut output).unwrap();
    }

    #[test]
    fn test_write_json() {
        let mut output = Output {
            name: String::from("format_test"),
            directory: String::from("./tmp"),
            format: String::from("jsonl"),
            compress: false,
            timeline: false,
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
        let json_line = create_line(&serde_json::Value::String(String::from("test"))).unwrap();
        write_json(json_line.as_bytes(), &mut output, &uuid).unwrap();
    }

    #[test]
    fn test_create_line() {
        let mut data = serde_json::Value::String(String::from("test"));

        let line = create_line(&mut data).unwrap();
        assert!(!line.is_empty());
    }
}
