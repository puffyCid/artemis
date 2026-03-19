use super::error::FormatError;
use crate::{
    artifacts::os::systeminfo::info::hostname,
    structs::toml::Output,
    utils::{logging::collection_status, output::final_output, uuid::generate_uuid},
};
use log::error;
use serde_json::Value;

/// Output data as csv
pub(crate) fn csv_format(
    serde_data: &Value,
    artifact_name: &str,
    output: &mut Output,
) -> Result<(), FormatError> {
    let uuid = generate_uuid();
    let filename = format!("{artifact_name}_{uuid}");

    if let Err(err) = final_output(serde_data, output, &filename) {
        error!("[forensics] Failed to output {artifact_name} csv: {err:?}");
        return Err(FormatError::Output);
    }

    let _ = collection_status(&hostname(), output, &filename);

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

        let collection_output = json![{
                "endpoint_id": "test",
                "id": "1",
                "artifact_name": "test",
                "complete_time": time_now(),
                "start_time": 1,

        }];

        csv_format(&collection_output, "test", &mut output).unwrap();
    }
}
