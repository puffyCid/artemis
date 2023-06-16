use std::fs::{remove_dir, remove_file};

use super::{artemis_toml::Output, compression::compress_output_zip, error::ArtemisError};
use crate::{
    filesystem::files::list_files,
    output::{
        local::output::local_output,
        remote::{aws::aws_upload, azure::azure_upload, gcp::gcp_upload},
    },
};
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
                    return Err(ArtemisError::Remote);
                }
            }
        }
        "azure" => {
            let azure_result = azure_upload(artifact_data, output, output_name);
            match azure_result {
                Ok(_) => {}
                Err(err) => {
                    error!("[artemis-core] Failed to upload to Azure Blog Storage: {err:?}");
                    return Err(ArtemisError::Remote);
                }
            }
        }
        "aws" => {
            let aws_result = aws_upload(artifact_data, output, output_name);
            match aws_result {
                Ok(_) => {}
                Err(err) => {
                    error!("[artemis-core] Failed to upload to AWS S3 Bucket: {err:?}");
                    return Err(ArtemisError::Remote);
                }
            }
        }
        _ => {
            warn!("Unknown output format: {}", output.format);
        }
    }
    Ok(())
}

/// Compress the local output directory to a zip file and delete any log/jsonl/json files
pub(crate) fn compress_final_output(output: &Output) -> Result<(), ArtemisError> {
    let output_dir = format!("{}/{}", output.directory, output.name);
    let zip_name = format!("{}/{}", output.directory, output.name);
    let zip_result = compress_output_zip(&output_dir, &zip_name);
    match zip_result {
        Ok(_) => {}
        Err(err) => {
            error!(
                "[artemis-core] Failed to zip output directory: {err:?}. DID NOT DELETE OUTPUT."
            );
            return Err(ArtemisError::Cleanup);
        }
    }

    /*
     * Now ready to delete output. Since we run in elevated privileges we need to be careful.
     * To maximize safety we only delete:
     *  - Files that end in .json, .jsonl, .log, or .gz
     *  - Only delete the output directory if its empty. Which means all the files above must be gone
     */
    let check = list_files(&output_dir);
    match check {
        Ok(results) => {
            for entry in results {
                if !entry.ends_with(".json") && !entry.ends_with(".log") && !entry.ends_with(".gz")
                {
                    continue;
                }
                // Remove our files. Entry is the full path to the file
                let _ = remove_file(&entry);
            }
        }
        Err(err) => {
            error!("[artemis-core] Failed to list files in output directory: {err:?}. DID NOT DELETE OUTPUT.");
            return Err(ArtemisError::Cleanup);
        }
    }
    // Now remove directory if its empty
    let remove_status = remove_dir(output_dir);
    match remove_status {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Failed to remove output directory: {err:?}");
            return Err(ArtemisError::Cleanup);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{compress_final_output, output_artifact};
    use crate::utils::artemis_toml::Output;
    use std::{fs::remove_file, path::PathBuf};

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

    #[test]
    fn test_compress_final_output() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system");

        let output = Output {
            name: String::from("files"),
            directory: test_location.display().to_string(),
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

        let _ = compress_final_output(&output);
        let _ = remove_file(format!("{}/files.zip", test_location.display().to_string())).unwrap();
    }
}
