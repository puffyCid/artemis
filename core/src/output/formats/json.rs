use super::error::FormatError;
use crate::{
    artifacts::os::systeminfo::info::get_info_metadata,
    structs::toml::Output,
    utils::{
        compression::compress::compress_gzip_data,
        logging::collection_status,
        output::final_output,
        time::{time_now, unixepoch_to_iso},
        uuid::generate_uuid,
    },
};
use log::{error, info};
use serde_json::{json, Value};

/// Output to `json` format with some metdata
pub(crate) fn json_format(
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
            "uuid": generate_uuid(),
            "id": output.collection_id,
            "artifact_name": output_name,
            "complete_time": unixepoch_to_iso(&(time_now() as i64)),
            "start_time": unixepoch_to_iso(&(*start_time as i64)),
            "hostname": info.hostname,
            "os_version": info.os_version,
            "platform": info.platform,
            "kernel_version": info.kernel_version,
            "load_performance": info.performance
        }
    }];

    collection_output["data"] = serde_data.clone();

    raw_json(&collection_output, output_name, output)
}

/// Output to `json` format
pub(crate) fn raw_json(
    serde_data: &Value,
    output_name: &str,
    output: &mut Output,
) -> Result<(), FormatError> {
    let serde_collection_results = serde_json::to_string(serde_data);
    let serde_collection = match serde_collection_results {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize json output: {err:?}");
            return Err(FormatError::Serialize);
        }
    };
    let collection_data = serde_collection.as_bytes();

    let output_data = if output.compress {
        let compressed_results = compress_gzip_data(collection_data);
        match compressed_results {
            Ok(result) => result,
            Err(err) => {
                error!("[artemis-core] Failed to compress data: {err:?}");
                return Err(FormatError::Output);
            }
        }
    } else {
        collection_data.to_vec()
    };

    let uuid = generate_uuid();
    let output_result = final_output(&output_data, output, &uuid);
    match output_result {
        Ok(_) => info!("[artemis-core] {} json output success", output_name),
        Err(err) => {
            error!("[artemis-core] Failed to output {output_name} json: {err:?}");
            return Err(FormatError::Output);
        }
    }
    let _ = collection_status(output_name, output, &uuid);

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
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("json"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        };
        let start_time = time_now();

        let name = "test";
        let data = serde_json::Value::String(String::from("test"));
        json_format(&data, name, &mut output, &start_time).unwrap();
    }

    #[test]
    fn test_raw_json() {
        let mut output = Output {
            name: String::from("format_test_raw"),
            directory: String::from("./tmp"),
            format: String::from("json"),
            compress: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("json"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        };

        let name = "test";
        let data = serde_json::Value::String(String::from("test123"));
        raw_json(&data, name, &mut output).unwrap();
    }
}
