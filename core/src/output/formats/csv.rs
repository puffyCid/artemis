use super::error::FormatError;
use crate::{
    structs::toml::Output,
    utils::{logging::collection_status, output::final_output, uuid::generate_uuid},
};
use csv::{Writer, WriterBuilder};
use log::{error, info};
use serde_json::Value;
use std::io::{Error, ErrorKind};

/// Output data as csv
pub(crate) fn csv_format(
    serde_data: &Value,
    output_name: &str,
    output: &mut Output,
) -> Result<(), FormatError> {
    let writer_result = csv_writer(serde_data);
    let writer = match writer_result {
        Ok(result) => result,
        Err(err) => {
            error!("[artemis-core] Could not create csv writer: {err:?}");
            return Err(FormatError::Output);
        }
    };

    let uuid = generate_uuid();

    let output_result: Result<_, _> =
        final_output(&writer.into_inner().unwrap_or_default(), output, &uuid);
    match output_result {
        Ok(_) => info!("[artemis-core] {output_name} jsonl output success"),
        Err(err) => {
            error!("[artemis-core] Failed to output {output_name} csv: {err:?}");
            return Err(FormatError::Output);
        }
    }

    let _ = collection_status(output_name, output, &uuid);

    Ok(())
}

/// Write serde data into a csv
fn csv_writer(serde_data: &Value) -> Result<Writer<Vec<u8>>, Error> {
    let mut writer = WriterBuilder::new().from_writer(Vec::new());

    let mut header = Vec::new();
    if serde_data.is_object() {
        for key in serde_data.as_object().unwrap().keys() {
            header.push(key);
        }
    } else if serde_data.is_array() {
        let row = serde_data.as_array().unwrap().first();
        if let Some(value) = row {
            if value.is_object() {
                for key in value.as_object().unwrap().keys() {
                    header.push(key);
                }
            }
        }
    }

    if header.is_empty() {
        return Err(Error::new(ErrorKind::InvalidData, "no headers"));
    }

    writer.write_record(&header)?;

    let mut rows = Vec::new();
    if serde_data.is_object() {
        for value in serde_data.as_object().unwrap().values() {
            rows.push(serde_json::to_string(value).unwrap_or_default());
        }

        writer.write_record(&rows)?;
    } else if serde_data.is_array() {
        for values in serde_data.as_array().unwrap() {
            let mut rows = Vec::new();

            if values.is_object() {
                for value in values.as_object().unwrap().values() {
                    rows.push(serde_json::to_string(value).unwrap_or_default());
                }
                writer.write_record(&rows)?;
            }
        }
    }

    Ok(writer)
}

#[cfg(test)]
mod tests {
    use super::{csv_format, csv_writer};
    use crate::{structs::toml::Output, utils::time::time_now};
    use serde_json::json;

    #[test]
    fn test_csv_format() {
        let mut output = Output {
            name: String::from("format_test"),
            directory: String::from("./tmp"),
            format: String::from("csv"),
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

        let collection_output = json![{
                "endpoint_id": "test",
                "id": "1",
                "artifact_name": "test",
                "complete_time": time_now(),
                "start_time": 1,

        }];

        csv_format(&collection_output, "test", &mut output).unwrap();
    }

    #[test]
    #[should_panic(expected = "InvalidData")]
    fn test_csv_writer() {
        let data = serde_json::Value::String(String::from("test"));
        let _ = csv_writer(&data).unwrap();
    }
}
