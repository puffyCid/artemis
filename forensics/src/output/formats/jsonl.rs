use super::{error::FormatError, timeline::timeline_data};
use crate::{
    artifacts::os::systeminfo::info::hostname,
    structs::toml::Output,
    utils::{logging::collection_status, output::final_output, uuid::generate_uuid},
};
use log::error;
use serde_json::Value;

/// Output to `jsonl` files
pub(crate) fn jsonl_format(
    serde_data: &mut Value,
    artifact_name: &str,
    output: &mut Output,
    start_time: u64,
) -> Result<(), FormatError> {
    // If our data is an array loop through each element and output as a separate line
    if serde_data.is_array() && output.timeline {
        // If we are timelining data. Timeline now before appending collection metadata
        timeline_data(serde_data, artifact_name);
    }
    let status = final_output(serde_data, output, artifact_name, start_time);
    if let Err(result) = status {
        error!("[forensics] Failed to output {artifact_name} data: {result:?}");
    }

    let uuid = generate_uuid();
    let filename = format!("{artifact_name}_{uuid}");

    let _ = collection_status(&hostname(), output, &filename);

    Ok(())
}

/// Output to `jsonl` files without metadata
pub(crate) fn raw_jsonl(
    serde_data: &mut Value,
    artifact_name: &str,
    output: &mut Output,
) -> Result<(), FormatError> {
    let uuid = generate_uuid();
    let filename = format!("{artifact_name}_{uuid}");
    let disable_metadata = 0;
    // If our data is an array loop through each element and output as a separate line
    if serde_data.is_array() {
        let empty_vec = Vec::new();
        let entries = serde_data.as_array().unwrap_or(&empty_vec);
        if entries.is_empty() {
            return Ok(());
        }
        let status = final_output(serde_data, output, artifact_name, disable_metadata);
        if let Err(result) = status {
            error!("[forensics] Failed to output {artifact_name} data: {result:?}");
        }
    } else {
        let status = final_output(serde_data, output, artifact_name, disable_metadata);
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
        let mut data = serde_json::Value::String(String::from("test"));
        raw_jsonl(&mut data, name, &mut output).unwrap();
    }
}
