use super::{artemis_toml::Output, error::ArtemisError};
use crate::output::{local::output::local_output, remote::gcp::gcp_upload};
use log::{error, warn};

/// Output artifact data based on output type
pub(crate) fn output_artifact(
    artifact_data: &[u8],
    output: &Output,
    output_name: &str,
) -> Result<(), ArtemisError> {
    // Check for supported output types. Can customize via Cargo.toml
    match output.output.as_str() {
        "local" => {
            let local_result = local_output(artifact_data, output, output_name, &output.format);
            match local_result {
                Ok(_) => {}
                Err(err) => {
                    error!("[artemis-core] Failed to output to local system: {err:?}");
                    return Err(ArtemisError::Local);
                }
            }
        }
        "gcp" => {
            let gcp_result = gcp_upload(artifact_data, output, output_name);
            match gcp_result {
                Ok(_) => {}
                Err(err) => {
                    error!("[artemis-core] Failed to upload to Google Cloud Storage: {err:?}");
                    return Err(ArtemisError::Gcp);
                }
            }
        }
        _ => {
            warn!("Unknown output format: {}", output.format);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::output_artifact;
    use crate::utils::artemis_toml::Output;

    #[test]
    fn test_output_artifact() {
        let output = Output {
            name: String::from("test_output"),
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
            output: String::from("local"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
        };

        let test = "A rust program";
        let name = "output";
        let result = output_artifact(test.as_bytes(), &output, name).unwrap();
        assert_eq!(result, ());
    }
}
