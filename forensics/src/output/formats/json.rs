use super::error::FormatError;
use crate::{structs::toml::Output, utils::output::final_output};
use log::error;
use serde_json::Value;

/// Output to `json` format with some metadata
pub(crate) fn json_format(
    serde_data: &mut Value,
    artifact_name: &str,
    output: &mut Output,
    start_time: u64,
) -> Result<(), FormatError> {
    let status = final_output(serde_data, output, artifact_name, start_time, false);
    if let Err(result) = status {
        error!("[forensics] Failed to output {artifact_name} data: {result:?}");
        return Err(FormatError::Output);
    }

    Ok(())
}

/// Output to `json` format
pub(crate) fn raw_json(
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
    use super::raw_json;
    use crate::{output::formats::json::json_format, structs::toml::Output, utils::time::time_now};

    #[test]
    fn test_json_format() {
        let mut output = Output {
            name: String::from("format_test"),
            directory: String::from("./tmp"),
            format: String::from("json"),
            compress: false,
            endpoint_id: String::from("abcd"),
            output: String::from("local"),
            ..Default::default()
        };
        let start_time = time_now();

        let name = "test";
        let mut data = serde_json::Value::String(String::from("test"));
        json_format(&mut data, name, &mut output, start_time).unwrap();
    }

    #[test]
    fn test_raw_json() {
        let mut output = Output {
            name: String::from("format_test_raw"),
            directory: String::from("./tmp"),
            format: String::from("json"),
            compress: false,
            endpoint_id: String::from("abcd"),
            output: String::from("local"),
            ..Default::default()
        };

        let name = "test";
        let mut data = serde_json::Value::String(String::from("test123"));
        raw_json(&mut data, name, &mut output).unwrap();
    }
}
