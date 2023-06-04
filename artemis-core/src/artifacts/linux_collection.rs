use crate::artifacts::applications::artifacts::{
    chromium_downloads, chromium_history, firefox_downloads, firefox_history,
};
use crate::artifacts::os::linux::artifacts::{files, processes, systeminfo};
use crate::artifacts::os::linux::error::LinuxArtifactError;
use crate::artifacts::os::unix::artifacts::{bash_history, cron_job, python_history, zsh_history};
use crate::filesystem::files::list_files;
use crate::utils::artemis_toml::ArtemisToml;
use crate::utils::compression::compress_output_zip;
use log::{error, info, warn};
use std::fs::{remove_dir, remove_file};

pub(crate) fn linux_collection(toml_data: &[u8]) -> Result<(), LinuxArtifactError> {
    let collector_results = ArtemisToml::parse_artemis_toml_data(toml_data);
    let mut collector = match collector_results {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Linux Artemis failed to parse TOML data: {err:?}");
            return Err(LinuxArtifactError::BadToml);
        }
    };

    for artifacts in collector.artifacts {
        let filter = artifacts.filter.unwrap_or(false);
        match artifacts.artifact_name.as_str() {
            "files" => {
                let file_data = artifacts.files;
                let file_artifact_config = match file_data {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = files(&file_artifact_config, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected file listing"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse file listing, error: {err:?}");
                        continue;
                    }
                }
            }
            "cron" => {
                let results = cron_job(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected macOS cron"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse macOS cron data, error: {err:?}");
                        continue;
                    }
                }
            }
            "shell_history" => {
                let results = bash_history(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Linux bash history"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse Linux bash history, error: {err:?}");
                        continue;
                    }
                }
                let results = zsh_history(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Linux zsh history"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse Linux zsh history, error: {err:?}");
                        continue;
                    }
                }
                let results = python_history(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Linux python history"),
                    Err(err) => {
                        error!(
                            "[artemis-core] Failed to parse Linux python history, error: {err:?}"
                        );
                        continue;
                    }
                }
            }
            "processes" => {
                let proc = artifacts.processes;
                let proc_artifacts = match proc {
                    Some(result) => result,
                    _ => continue,
                };

                let results = processes(&proc_artifacts, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected processes"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse processes, error: {err:?}");
                        continue;
                    }
                }
            }
            "firefox-history" => {
                let results = firefox_history(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Firefox history"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse Firefox history, error: {err:?}");
                        continue;
                    }
                }
            }
            "firefox-downloads" => {
                let results = firefox_downloads(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Firefox downloads"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse Firefox downloads, error: {err:?}");
                        continue;
                    }
                }
            }
            "chromium-history" => {
                let results = chromium_history(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Chromium history"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse Chromium history, error: {err:?}");
                        continue;
                    }
                }
            }
            "chromium-downloads" => {
                let results = chromium_downloads(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Chromium downloads"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse Chromium downloads, error: {err:?}");
                        continue;
                    }
                }
            }
            "systeminfo" => {
                let results = systeminfo(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected systeminfo"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse system, error: {err:?}");
                        continue;
                    }
                }
            }
            _ => warn!(
                "[artemis-core] Unsupported Linux artifact: {}",
                artifacts.artifact_name
            ),
        }
    }
    if collector.output.compress && collector.output.output == "local" {
        let output_dir = format!("{}/{}", collector.output.directory, collector.output.name);
        let zip_name = collector.output.name;
        let zip_result = compress_output_zip(&output_dir, &zip_name);
        match zip_result {
            Ok(_) => {}
            Err(err) => {
                error!("[artemis-core] Failed to zip output directory: {err:?}. DID NOT DELETE OUTPUT.");
                return Err(LinuxArtifactError::Cleanup);
            }
        }

        /*
         * Now ready to delete output. Since we run in elevated privileges this is kind of terrifying.
         * To maximize safety we only delete:
         *  - Files that end in .json, .jsonl, .log, or .gz
         *  - Only delete the output directory if its empty. Which means all the files above must be gone
         */
        let check = list_files(&output_dir);
        match check {
            Ok(results) => {
                for entry in results {
                    if !entry.ends_with(".json")
                        && !entry.ends_with(".log")
                        && !entry.ends_with(".gz")
                    {
                        continue;
                    }
                    // Remove our files. Entry is the full path to the file
                    let _ = remove_file(&entry);
                }
            }
            Err(err) => {
                error!("[artemis-core] Failed to list files in output directory: {err:?}. DID NOT DELETE OUTPUT.");
                return Err(LinuxArtifactError::Cleanup);
            }
        }
        // Now remove directory if its empty
        let remove_status = remove_dir(output_dir);
        match remove_status {
            Ok(_) => {}
            Err(err) => {
                error!("[artemis-core] Failed to remove empty output directory: {err:?}");
                return Err(LinuxArtifactError::Cleanup);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::artifacts::linux_collection::linux_collection;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_linux_collection() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/quick.toml");

        let mut buffer = read_file(&test_location.display().to_string()).unwrap();
        linux_collection(&buffer).unwrap();
    }
}
