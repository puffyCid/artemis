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

/// Output to `json` format
pub(crate) fn json_format(
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
            "uuid": generate_uuid(),
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

    collection_output["data"] = serde_data.clone();

    let serde_collection_results = serde_json::to_string(&collection_output);
    let serde_collection = match serde_collection_results {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize json output: {err:?}");
            return Err(FormatError::Serialize);
        }
    };

    let uuid = generate_uuid();
    let output_result = output_artifact(serde_collection.as_bytes(), output, &uuid);
    match output_result {
        Ok(_) => info!("[artemis-core] {} json output success", output_name),
        Err(err) => {
            error!("[artemis-core] Failed to output {output_name} json: {err:?}");
            return Err(FormatError::Output);
        }
    }

    if output.compress {
        let path = format!("{}/{}/{}.json", output.directory, output.name, uuid);
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

#[cfg(test)]
mod tests {
    use crate::{
        output::formats::json::json_format,
        utils::{artemis_toml::Output, time::time_now},
    };

    #[test]
    fn test_output_data() {
        let mut output = Output {
            name: String::from("format_test"),
            directory: String::from("./tmp"),
            format: String::from("json"),
            compress: false,
            url: Some(String::new()),
            port: Some(0),
            api_key: Some(String::new()),
            username: Some(String::new()),
            password: Some(String::new()),
            generic_keys: Some(Vec::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("json"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
        };
        let start_time = time_now();

        let name = "test";
        let data = serde_json::Value::String(String::from("test"));
        json_format(&data, name, &mut output, &start_time).unwrap();
    }
}
