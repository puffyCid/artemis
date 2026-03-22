use super::{error::FormatError, timeline::timeline_data};
use crate::{structs::toml::Output, utils::output::final_output};
use log::error;
use serde_json::Value;

/// Output to `jsonl` files
pub(crate) fn jsonl_format(
    serde_data: &mut Value,
    artifact_name: &str,
    output: &mut Output,
    start_time: u64,
) -> Result<(), FormatError> {
    // Check if we want to timeline data. Only array of JSON objects can be timelined
    if serde_data.is_array() && output.timeline {
        // If we are timelining data. Timeline now before appending collection metadata
        timeline_data(serde_data, artifact_name);
    }
    let status = final_output(serde_data, output, artifact_name, start_time, false);
    if let Err(result) = status {
        error!("[forensics] Failed to output {artifact_name} data: {result:?}");
        return Err(FormatError::Output);
    }

    Ok(())
}

/// Output to `jsonl` files without metadata
pub(crate) fn raw_jsonl(
    serde_data: &mut Value,
    artifact_name: &str,
    output: &mut Output,
) -> Result<(), FormatError> {
    let disable_metadata = 0;
    let status = final_output(serde_data, output, artifact_name, disable_metadata, false);
    if let Err(result) = status {
        error!("[forensics] Failed to output {artifact_name} data: {result:?}");
        return Err(FormatError::Output);
    }

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
