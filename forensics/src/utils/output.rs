use super::error::ArtemisError;
use crate::output::local::output::local_output;
use crate::output::remote::api::api_upload;
use crate::utils::compression::compress::compress_output_zip;
use crate::utils::logging::collection_status;
use crate::utils::uuid::generate_uuid;
use crate::{
    filesystem::files::list_files,
    output::remote::{aws::aws_upload, azure::azure_upload, gcp::gcp_upload},
    structs::toml::Output,
};
use log::{error, warn};
use serde_json::Value;
use std::fs::{remove_dir, remove_file};

/// Output artifact data based on output type
pub(crate) fn final_output(
    data: &mut Value,
    output: &mut Output,
    artifact_name: &str,
    start_time: u64,
    is_logs: bool,
) -> Result<(), ArtemisError> {
    // If the logs are getting uploaded. Do not append uuid data
    let filename = if is_logs {
        artifact_name.to_string()
    } else {
        let uuid = generate_uuid();
        format!("{artifact_name}_{uuid}")
    };

    // Check for supported output types. Can customize via Cargo.toml
    match output.output.as_str() {
        "local" => match local_output(data, output, &filename, start_time, artifact_name) {
            Ok(_) => {}
            Err(err) => {
                error!("[forensics] Failed to output to local system: {err:?}");
                return Err(ArtemisError::Local);
            }
        },
        "gcp" => match gcp_upload(data, output, &filename, start_time, artifact_name) {
            Ok(_) => {}
            Err(err) => {
                error!("[forensics] Failed to upload to Google Cloud Storage: {err:?}");
                return Err(ArtemisError::Remote);
            }
        },
        "aws" => match aws_upload(data, output, &filename, start_time, artifact_name) {
            Ok(_) => {}
            Err(err) => {
                error!("[forensics] Failed to upload to AWS S3 Bucket: {err:?}");
                return Err(ArtemisError::Remote);
            }
        },
        "azure" => match azure_upload(data, output, &filename, start_time, artifact_name) {
            Ok(_) => {}
            Err(err) => {
                error!("[forensics] Failed to upload to Azure Blob Storage: {err:?}");
                return Err(ArtemisError::Remote);
            }
        },
        "api" => match api_upload(data, output, &filename, start_time, artifact_name) {
            Ok(_) => {}
            Err(err) => {
                error!("[forensics] Failed to upload to API server: {err:?}");
                return Err(ArtemisError::Remote);
            }
        },
        _ => {
            warn!("Unknown output format: {}", output.format);
        }
    }

    if !is_logs {
        let _ = collection_status(output, &filename);
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
            error!("[forensics] Failed to zip output directory: {err:?}. DID NOT DELETE OUTPUT.");
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
                    && !entry.ends_with(".zip")
                {
                    continue;
                }
                // Remove our files. Entry is the full path to the file
                let _ = remove_file(&entry);
            }
        }
        Err(err) => {
            error!(
                "[forensics] Failed to list files in output directory: {err:?}. DID NOT DELETE OUTPUT."
            );
            return Err(ArtemisError::Cleanup);
        }
    }
    // Now remove directory if its empty
    let remove_status = remove_dir(output_dir);
    match remove_status {
        Ok(_) => {}
        Err(err) => {
            error!("[forensics] Failed to remove output directory: {err:?}");
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
        let mut output = Output {
            name: String::from("test_output"),
            directory: String::from("./tmp"),
            format: String::from("json"),
            endpoint_id: String::from("abcd"),
            output: String::from("local"),
            ..Default::default()
        };

        let test = "A rust program";
        let name = "output";
        let result = final_output(
            &mut serde_json::to_value(&test).unwrap(),
            &mut output,
            name,
            0,
            false,
        )
        .unwrap();
        assert_eq!(result, ());
    }

    #[test]
    fn test_no_output() {
        let mut output = Output {
            name: String::from("no_output"),
            directory: String::from("./tmp"),
            format: String::from("json"),
            endpoint_id: String::from("abcd"),
            output: String::from("none"),
            ..Default::default()
        };

        let test = "A rust program";
        let name = "output";
        let result = final_output(
            &mut serde_json::to_value(&test).unwrap(),
            &mut output,
            name,
            0,
            false,
        )
        .unwrap();
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
            endpoint_id: String::from("abcd"),
            output: String::from("local"),
            ..Default::default()
        };

        let _ = compress_final_output(&output);
        let _ = remove_file(format!("{}/files.zip", test_location.display().to_string())).unwrap();
    }
}
