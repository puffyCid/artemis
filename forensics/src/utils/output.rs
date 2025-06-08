use super::error::ArtemisError;
use crate::output::remote::api::api_upload;
use crate::utils::compression::compress::compress_output_zip;
use crate::{
    filesystem::files::list_files,
    output::{
        local::output::local_output,
        remote::{aws::aws_upload, azure::azure_upload, gcp::gcp_upload},
    },
    structs::toml::Output,
};
use log::{error, warn};
use std::fs::{remove_dir, remove_file};

/// Output artifact data based on output type
pub(crate) fn final_output(
    artifact_data: &[u8],
    output: &Output,
    output_name: &str,
) -> Result<(), ArtemisError> {
    // Check for supported output types. Can customize via Cargo.toml
    match output.output.as_str() {
        "local" => match local_output(artifact_data, output, output_name, &output.format) {
            Ok(_) => {}
            Err(err) => {
                error!("[core] Failed to output to local system: {err:?}");
                return Err(ArtemisError::Local);
            }
        },
        "gcp" => match gcp_upload(artifact_data, output, output_name) {
            Ok(_) => {}
            Err(err) => {
                error!("[core] Failed to upload to Google Cloud Storage: {err:?}");
                return Err(ArtemisError::Remote);
            }
        },
        "azure" => match azure_upload(artifact_data, output, output_name) {
            Ok(_) => {}
            Err(err) => {
                error!("[core] Failed to upload to Azure Blog Storage: {err:?}");
                return Err(ArtemisError::Remote);
            }
        },
        "aws" => match aws_upload(artifact_data, output, output_name) {
            Ok(_) => {}
            Err(err) => {
                error!("[core] Failed to upload to AWS S3 Bucket: {err:?}");
                return Err(ArtemisError::Remote);
            }
        },
        "api" => match api_upload(artifact_data, output, &false) {
            Ok(_) => {}
            Err(err) => {
                error!("[core] Failed to upload to API server: {err:?}");
                return Err(ArtemisError::Remote);
            }
        },
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
            error!("[core] Failed to zip output directory: {err:?}. DID NOT DELETE OUTPUT.");
            return Err(ArtemisError::Cleanup);
        }
    }

    /*
     * Now ready to delete output. Since we often run in elevated privileges we need to be careful.
     * To maximize safety we only delete:
     *  - Files that end in .json, .jsonl, .log, .gz, or .csv
     *  - Also we only delete the output directory if its empty. Which means all the files above must be gone
     */
    let check = list_files(&output_dir);
    match check {
        Ok(results) => {
            for entry in results {
                if !entry.ends_with(".json")
                    && !entry.ends_with(".log")
                    && !entry.ends_with(".gz")
                    && !entry.ends_with(".csv")
                    && !entry.ends_with(".jsonl")
                {
                    continue;
                }
                // Remove our files. Entry is the full path to the file
                let _ = remove_file(&entry);
            }
        }
        Err(err) => {
            error!(
                "[core] Failed to list files in output directory: {err:?}. DID NOT DELETE OUTPUT."
            );
            return Err(ArtemisError::Cleanup);
        }
    }
    // Now remove directory if its empty
    let remove_status = remove_dir(output_dir);
    match remove_status {
        Ok(_) => {}
        Err(err) => {
            error!("[core] Failed to remove output directory: {err:?}");
            return Err(ArtemisError::Cleanup);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{compress_final_output, final_output};
    use crate::structs::toml::Output;
    use std::{fs::remove_file, path::PathBuf};

    #[test]
    fn test_final_output() {
        let output = Output {
            name: String::from("test_output"),
            directory: String::from("./tmp"),
            format: String::from("json"),
            compress: false,
            timeline: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        };

        let test = "A rust program";
        let name = "output";
        let result = final_output(test.as_bytes(), &output, name).unwrap();
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
            timeline: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: String::from("local"),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        };

        let _ = compress_final_output(&output);
        let _ = remove_file(format!("{}/files.zip", test_location.display().to_string())).unwrap();
    }
}
