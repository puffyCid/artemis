use crate::artifacts::applications::artifacts::{
    chromium_downloads, chromium_history, firefox_downloads, firefox_history,
};
use crate::artifacts::os::linux::artifacts::{files, processes, systeminfo};
use crate::artifacts::os::linux::error::LinuxArtifactError;
use crate::artifacts::os::unix::artifacts::{bash_history, cron_job, python_history, zsh_history};
use crate::runtime::deno::execute_script;
use crate::structs::toml::ArtemisToml;
use crate::utils::{logging::upload_logs, output::compress_final_output};
use log::{error, info, warn};

use super::os::linux::artifacts::{journals, logons};
use super::os::unix::artifacts::sudo_logs;

/// Parse the TOML collector and get Linux artifact targets
pub(crate) fn linux_collection(collector: &mut ArtemisToml) -> Result<(), LinuxArtifactError> {
    for artifacts in &collector.artifacts {
        let filter = artifacts.filter.unwrap_or(false);
        match artifacts.artifact_name.as_str() {
            "files" => {
                let file_data = &artifacts.files;
                let file_artifact_config = match file_data {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = files(file_artifact_config, &mut collector.output, &filter);
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
                    Ok(_) => info!("Collected Linux cron"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse Linux cron data, error: {err:?}");
                        continue;
                    }
                }
            }
            "sudologs" => {
                let results = sudo_logs(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Linux sudo logs"),
                    Err(err) => {
                        error!(
                            "[artemis-core] Failed to parse Linux sudo log data, error: {err:?}"
                        );
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
                let proc = &artifacts.processes;
                let proc_artifacts = match proc {
                    Some(result) => result,
                    _ => continue,
                };

                let results = processes(proc_artifacts, &mut collector.output, &filter);
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
            "journal" => {
                let results = journals(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected journals"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse journals, error: {err:?}");
                        continue;
                    }
                }
            }
            "script" => {
                let script_data = &artifacts.script;
                let script = match script_data {
                    Some(result) => result,
                    _ => continue,
                };
                let results = execute_script(&mut collector.output, script);
                match results {
                    Ok(_) => info!("Executed JavaScript "),
                    Err(err) => {
                        error!("[artemis-core] Failed to execute JavaScript error: {err:?}");
                        continue;
                    }
                }
            }
            "logon" => {
                let results = logons(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected logons"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse logons, error: {err:?}");
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

    if collector.output.output != "local" {
        let output_dir = format!("{}/{}", collector.output.directory, collector.output.name);

        let _ = upload_logs(&output_dir, &collector.output);
    } else if collector.output.compress && collector.output.output == "local" {
        let _ = compress_final_output(&collector.output);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::artifacts::linux_collection::linux_collection;
    use crate::filesystem::files::read_file;
    use crate::structs::toml::ArtemisToml;
    use std::path::PathBuf;

    #[test]
    fn test_linux_collection() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/quick.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let mut collector = ArtemisToml::parse_artemis_toml(&buffer).unwrap();
        linux_collection(&mut collector).unwrap();
    }
}
