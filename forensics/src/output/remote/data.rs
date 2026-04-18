use crate::{
    artifacts::os::systeminfo::info::get_info_metadata,
    output::remote::error::RemoteError,
    structs::toml::Output,
    utils::{
        compression::compress::compress_gzip_bytes,
        time::{time_now, unixepoch_to_iso},
        uuid::generate_uuid,
    },
};
use log::error;
use serde_json::{Value, json};

/// Prepare parsed data for uploading to remote services
pub(crate) fn prep_data_upload(
    serde_data: &mut Value,
    output: &Output,
    remote: &str,
    artifact_name: &str,
    start_time: u64,
) -> Result<Vec<u8>, RemoteError> {
    let mut data = Vec::new();
    let uuid = generate_uuid();
    // Get small amount of system metadata
    let info = get_info_metadata();
    let complete = unixepoch_to_iso(time_now() as i64);
    let disable_meta = 0;

    // Write serde data as newline json
    if serde_data.is_array() {
        let value = serde_data.as_array_mut().unwrap();
        if value.is_empty() {
            let collection_output = json![{
                    "endpoint_id": output.endpoint_id,
                    "id": output.collection_id,
                    "uuid": uuid,
                    "artifact_name": artifact_name,
                    "complete_time": complete,
                    "start_time": unixepoch_to_iso(start_time as i64),
                    "hostname": info.hostname,
                    "os_version": info.os_version,
                    "platform": info.platform,
                    "kernel_version": info.kernel_version,
                    "load_performance": info.performance,
                    "artemis_version": info.artemis_version,
                    "rust_version": info.rust_version,
                    "build_date": info.build_date,
                    "interfaces": info.interfaces,
            }];

            data = serde_json::to_vec(&collection_output).unwrap_or_default();
        }

        for entry in value {
            // Append metadata row
            if entry.is_object() && start_time != disable_meta {
                entry["collection_metadata"] = json![{
                        "endpoint_id": output.endpoint_id,
                        "uuid": uuid,
                        "id": output.collection_id,
                        "artifact_name": artifact_name,
                        "complete_time": complete,
                        "start_time": unixepoch_to_iso(start_time as i64),
                        "hostname": info.hostname,
                        "os_version": info.os_version,
                        "platform": info.platform,
                        "kernel_version": info.kernel_version,
                        "load_performance": info.performance,
                        "artemis_version": info.artemis_version,
                        "rust_version": info.rust_version,
                        "build_date": info.build_date,
                        "interfaces": info.interfaces,
                }];
            }
            if let Err(err) = serde_json::to_writer(&mut data, entry) {
                error!("[forensics] Could not serialize to jsonl {remote} writer: {err:?}");
            }
            data.push(b'\n');
        }
    } else if serde_data.is_object() {
        if start_time != disable_meta {
            serde_data["collection_metadata"] = json![{
                    "endpoint_id": output.endpoint_id,
                    "uuid": uuid,
                    "id": output.collection_id,
                    "artifact_name": artifact_name,
                    "complete_time": complete,
                    "start_time": unixepoch_to_iso(start_time as i64),
                    "hostname": info.hostname,
                    "os_version": info.os_version,
                    "platform": info.platform,
                    "kernel_version": info.kernel_version,
                    "load_performance": info.performance,
                    "artemis_version": info.artemis_version,
                    "rust_version":info.rust_version,
                    "build_date": info.build_date,
                    "interfaces": info.interfaces,
            }];
        }
        if let Err(err) = serde_json::to_writer(&mut data, serde_data) {
            error!("[forensics] Could not serialize to json {remote} writer: {err:?}");
            return Err(RemoteError::RemoteUpload);
        }
    } else if let Err(err) = serde_json::to_writer(&mut data, serde_data) {
        error!("[forensics] Could not serialize to data {remote} writer: {err:?}");
        return Err(RemoteError::RemoteUpload);
    }

    if output.compress {
        data = match compress_gzip_bytes(&data) {
            Ok(result) => result,
            Err(_err) => return Err(RemoteError::RemoteUpload),
        }
    }

    Ok(data)
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::systeminfo::info::get_info, output::remote::data::prep_data_upload,
        structs::toml::Output,
    };
    use serde_json::{Value, json};
    #[test]
    fn test_prep_upload() {
        let out = Output::default();
        let mut test = Value::Null;
        let value = prep_data_upload(&mut test, &out, "test", "test", 2).unwrap();
        assert!(!value.is_empty());
    }

    #[test]
    fn test_prep_update_info() {
        let out = Output {
            format: String::from("jsonl"),
            ..Default::default()
        };
        let info = get_info();
        let mut value = serde_json::to_value(info).unwrap();
        let value = prep_data_upload(&mut value, &out, "test", "test", 2).unwrap();
        assert!(!value.is_empty());
    }

    #[test]
    fn test_prep_update_info_array() {
        let out = Output {
            format: String::from("jsonl"),
            ..Default::default()
        };
        let info = get_info();
        let mut value = json!([serde_json::to_value(info).unwrap()]);
        let value = prep_data_upload(&mut value, &out, "test", "test", 2).unwrap();
        assert!(!value.is_empty());
    }
}
