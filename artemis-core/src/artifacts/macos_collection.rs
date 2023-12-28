use super::{
    applications::artifacts::{
        chromium_downloads, chromium_history, firefox_downloads, firefox_history, safari_downloads,
        safari_history,
    },
    os::{
        macos::artifacts::{execpolicy, groups, processes, systeminfo, unifiedlogs, users},
        unix::artifacts::{bash_history, cron_job, python_history, sudo_logs},
    },
    os::{macos::error::MacArtifactError, unix::artifacts::zsh_history},
};
use crate::{
    artifacts::os::macos::artifacts::{emond, files, fseventsd, launchd, loginitems},
    runtime::deno::execute_script,
    structs::toml::ArtemisToml,
    utils::{logging::upload_logs, output::compress_final_output},
};
use log::{error, info, warn};

/// Parse the TOML collector and get macOS artifact targets
pub(crate) fn macos_collection(collector: &mut ArtemisToml) -> Result<(), MacArtifactError> {
    // Loop through all supported macOS artifacts
    for artifacts in &collector.artifacts {
        let filter = artifacts.filter.unwrap_or(false);
        match artifacts.artifact_name.as_str() {
            "loginitems" => {
                let options = match &artifacts.loginitems {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = loginitems(&mut collector.output, &filter, options);
                match results {
                    Ok(_) => info!("Collected loginitems"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse loginitems, error: {err:?}");
                        continue;
                    }
                }
            }
            "emond" => {
                let options = match &artifacts.emond {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = emond(&mut collector.output, &filter, options);
                match results {
                    Ok(_) => info!("Collected emond"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse emond, error: {err:?}");
                        continue;
                    }
                }
            }
            "fseventsd" => {
                let options = match &artifacts.fseventsd {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = fseventsd(&mut collector.output, &filter, options);
                match results {
                    Ok(_) => info!("Collected fseventsd"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse fseventsd, error: {err:?}");
                        continue;
                    }
                }
            }
            "launchd" => {
                let options = match &artifacts.launchd {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = launchd(&mut collector.output, &filter, options);
                match results {
                    Ok(_) => info!("Collected launchd"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse launchd, error: {err:?}");
                        continue;
                    }
                }
            }
            "files" => {
                let options = match &artifacts.files {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = files(&mut collector.output, &filter, options);
                match results {
                    Ok(_) => info!("Collected file listing"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse file listing, error: {err:?}");
                        continue;
                    }
                }
            }
            "users" => {
                let options = match &artifacts.users {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = users(&mut collector.output, &filter, options);
                match results {
                    Ok(_) => info!("Collected users"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse users, error: {err:?}");
                        continue;
                    }
                }
            }
            "groups" => {
                let options = match &artifacts.groups {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = groups(&mut collector.output, &filter, options);
                match results {
                    Ok(_) => info!("Collected groups"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse groups, error: {err:?}");
                        continue;
                    }
                }
            }
            "processes" => {
                let options = match &artifacts.processes {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = processes(&mut collector.output, &filter, options);
                match results {
                    Ok(_) => info!("Collected processes"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse processes, error: {err:?}");
                        continue;
                    }
                }
            }
            "systeminfo" => {
                let results = systeminfo(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected systeminfo"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse systeminfo, error: {err:?}");
                        continue;
                    }
                }
            }
            "execpolicy" => {
                let options = match &artifacts.execpolicy {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = execpolicy(&mut collector.output, &filter, options);
                match results {
                    Ok(_) => info!("Collected execpolicy"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse execpolicy, error: {err:?}");
                        continue;
                    }
                }
            }
            "unifiedlogs" => {
                let options = match &artifacts.unifiedlogs {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = unifiedlogs(&mut collector.output, &filter, options);
                match results {
                    Ok(_) => info!("Collected unified logs"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse unified logs, error: {err:?}");
                        continue;
                    }
                }
            }
            "safari-history" => {
                let results = safari_history(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Safari history"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse Safari history, error: {err:?}");
                        continue;
                    }
                }
            }
            "safari-downloads" => {
                let results = safari_downloads(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Safari downloads"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse Safari downloads, error: {err:?}");
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
            "shell_history" => {
                let results = bash_history(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected macOS bash history"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse macOS bash history, error: {err:?}");
                        continue;
                    }
                }
                let results = zsh_history(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected macOS zsh history"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse macOS zsh history, error: {err:?}");
                        continue;
                    }
                }
                let results = python_history(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected macOS python history"),
                    Err(err) => {
                        error!(
                            "[artemis-core] Failed to parse macOS python history, error: {err:?}"
                        );
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
            "sudologs" => {
                let results = sudo_logs(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected macOS sudo logs"),
                    Err(err) => {
                        error!(
                            "[artemis-core] Failed to parse macOS sudo log data, error: {err:?}"
                        );
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
            _ => warn!(
                "[artemis-core] Unsupported macOS artifact: {}",
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
    use super::macos_collection;
    use crate::{filesystem::files::read_file, structs::toml::ArtemisToml};
    use std::path::PathBuf;

    #[test]
    fn test_macos_collection() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/quick.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let mut collector = ArtemisToml::parse_artemis_toml(&buffer).unwrap();
        macos_collection(&mut collector).unwrap();
    }
}
