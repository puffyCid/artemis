use crate::{
    output::remote::error::RemoteError, structs::toml::Output,
    utils::compression::compress::compress_gzip_bytes,
};
use log::error;
use serde_json::Value;

/// Prepare parsed data for uploading to remote services
pub(crate) fn prep_data_upload(
    serde_data: &Value,
    output: &Output,
    remote: &str,
) -> Result<Vec<u8>, RemoteError> {
    let mut data = Vec::new();
    // Write serde data as newline json
    if serde_data.is_array() && output.format.to_lowercase() == "jsonl" {
        let value = serde_data.as_array().unwrap();
        for entry in value {
            if let Err(err) = serde_json::to_writer(&mut data, entry) {
                error!("[forensics] Could not serialize to jsonl {remote} writer: {err:?}");
            }
            data.push(b'\n');
        }
    } else if let Err(err) = serde_json::to_writer(&mut data, serde_data) {
        error!("[forensics] Could not serialize to json {remote} writer: {err:?}");
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
    use crate::{output::remote::data::prep_data_upload, structs::toml::Output};
    use serde_json::Value;

    #[test]
    fn test_prep_upload() {
        let out = Output::default();
        let test = Value::Null;
        let value = prep_data_upload(&test, &out, "test").unwrap();
        assert!(!value.is_empty());
    }
}
