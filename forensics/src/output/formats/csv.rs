use super::error::FormatError;
use crate::{structs::toml::Output, utils::output::final_output};
use log::error;
use serde_json::Value;

/// Output data as csv
pub(crate) fn csv_format(
    serde_data: &mut Value,
    artifact_name: &str,
    output: &mut Output,
) -> Result<(), FormatError> {
    if let Err(err) = final_output(serde_data, output, artifact_name, 0, false) {
        error!("[forensics] Failed to output {artifact_name} csv: {err:?}");
        return Err(FormatError::Output);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::csv_format;
    use crate::{structs::toml::Output, utils::time::time_now};
    use serde_json::json;

    #[test]
    fn test_csv_format() {
        let mut output = Output {
            name: String::from("format_test"),
            directory: String::from("./tmp"),
            format: String::from("csv"),
            compress: false,
            endpoint_id: String::from("abcd"),
            output: String::from("local"),
            ..Default::default()
        };

        let mut collection_output = json![{
                "endpoint_id": "test",
                "id": "1",
                "artifact_name": "test",
                "complete_time": time_now(),
                "start_time": 1,

        }];

        csv_format(&mut collection_output, "test", &mut output).unwrap();
    }
}
