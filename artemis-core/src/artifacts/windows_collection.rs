use super::{
    applications::artifacts::{
        chromium_downloads, chromium_history, firefox_downloads, firefox_history,
    },
    os::windows::{
        artifacts::{
            amcache, bits, eventlogs, files, jumplists, prefetch, processes, raw_filelist,
            registry, search, services, shellbags, shimcache, shimdb, shortcuts, srum, systeminfo,
            tasks, userassist, users, usnjrnl,
        },
        error::WinArtifactError,
    },
};
use crate::{
    runtime::deno::execute_script,
    utils::{artemis_toml::ArtemisToml, logging::upload_logs, output::compress_final_output},
};
use log::{error, info, warn};

/// Parse a Windows collection TOML script
pub(crate) fn windows_collection(toml_data: &[u8]) -> Result<(), WinArtifactError> {
    let collector_results = ArtemisToml::parse_artemis_toml_data(toml_data);
    let mut collector = match collector_results {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Windows Artemis failed to parse TOML data: {err:?}");
            return Err(WinArtifactError::BadToml);
        }
    };

    for artifacts in collector.artifacts {
        let filter = artifacts.filter.unwrap_or(false);

        match artifacts.artifact_name.as_str() {
            "prefetch" => {
                let artifact = match &artifacts.prefetch {
                    Some(result) => result,
                    None => continue,
                };
                let results = prefetch(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected prefetch"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse prefetch, error: {err:?}");
                        continue;
                    }
                }
            }
            "eventlogs" => {
                let artifact = match &artifacts.eventlogs {
                    Some(result) => result,
                    None => continue,
                };
                let results = eventlogs(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Eventlogs"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse Eventlogs, error: {err:?}");
                        continue;
                    }
                }
            }
            "rawfiles" => {
                let artifact = match &artifacts.rawfiles {
                    Some(result) => result,
                    None => continue,
                };
                let results = raw_filelist(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Raw Filelisting"),
                    Err(err) => {
                        error!("[artemis-core] Failed to get raw filelisting, error: {err:?}");
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
                        error!("[artemis-core] Failed to parse Windows processes, error: {err:?}");
                        continue;
                    }
                }
            }
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
            "systeminfo" => {
                let results = systeminfo(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected systeminfo"),
                    Err(err) => {
                        error!("[artemis-core] Failed to collect systeminfo, error: {err:?}");
                        continue;
                    }
                }
            }
            "shimdb" => {
                let artifact = match &artifacts.shimdb {
                    Some(result) => result,
                    None => continue,
                };
                let results = shimdb(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected shimdb"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse shimdb, error: {err:?}");
                        continue;
                    }
                }
            }
            "registry" => {
                let artifact = match &artifacts.registry {
                    Some(result) => result,
                    None => continue,
                };
                let results = registry(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected registry"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse registry, error: {err:?}");
                        continue;
                    }
                }
            }
            "userassist" => {
                let artifact = match &artifacts.userassist {
                    Some(result) => result,
                    None => continue,
                };
                let results = userassist(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected userassist"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse userassist, error: {err:?}");
                        continue;
                    }
                }
            }
            "shimcache" => {
                let artifact = match &artifacts.shimcache {
                    Some(result) => result,
                    None => continue,
                };
                let results = shimcache(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected shimcache"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse shimcache, error: {err:?}");
                        continue;
                    }
                }
            }
            "shellbags" => {
                let artifact = match &artifacts.shellbags {
                    Some(result) => result,
                    None => continue,
                };
                let results = shellbags(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected shellbags"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse shellbags, error: {err:?}");
                        continue;
                    }
                }
            }
            "amcache" => {
                let artifact = match &artifacts.amcache {
                    Some(result) => result,
                    None => continue,
                };
                let results = amcache(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected amcache"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse amcache, error: {err:?}");
                        continue;
                    }
                }
            }
            "script" => {
                let script_data = artifacts.script;
                let script = match script_data {
                    Some(result) => result,
                    _ => continue,
                };
                let results = execute_script(&mut collector.output, &script);
                match results {
                    Ok(_) => info!("Executed JavaScript "),
                    Err(err) => {
                        error!("[artemis-core] Failed to execute JavaScript error: {err:?}");
                        continue;
                    }
                }
            }
            "shortcuts" => {
                let artifact = match &artifacts.shortcuts {
                    Some(result) => result,
                    None => continue,
                };
                let results = shortcuts(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected shortcuts"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse shortcut files, error: {err:?}");
                        continue;
                    }
                }
            }
            "usnjrnl" => {
                let artifact = match &artifacts.usnjrnl {
                    Some(result) => result,
                    None => continue,
                };
                let results = usnjrnl(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected usnjrnl"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse usnjrnl, error: {err:?}");
                        continue;
                    }
                }
            }
            "bits" => {
                let artifact = match &artifacts.bits {
                    Some(result) => result,
                    None => continue,
                };
                let results = bits(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected bits"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse bits, error: {err:?}");
                        continue;
                    }
                }
            }
            "srum" => {
                let artifact = match &artifacts.srum {
                    Some(result) => result,
                    None => continue,
                };
                let results = srum(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected SRUM"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse srum, error: {err:?}");
                        continue;
                    }
                }
            }
            "search" => {
                let artifact = match &artifacts.search {
                    Some(result) => result,
                    None => continue,
                };
                let results = search(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected search"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse search, error: {err:?}");
                        continue;
                    }
                }
            }
            "users" => {
                let artifact = match &artifacts.users {
                    Some(result) => result,
                    None => continue,
                };
                let results = users(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Users"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse users, error: {err:?}");
                        continue;
                    }
                }
            }
            "tasks" => {
                let artifact = match &artifacts.tasks {
                    Some(result) => result,
                    None => continue,
                };
                let results = tasks(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Schedule Tasks"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse schedule tasks, error: {err:?}");
                        continue;
                    }
                }
            }
            "services" => {
                let artifact = match &artifacts.services {
                    Some(result) => result,
                    None => continue,
                };
                let results = services(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Services"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse services, error: {err:?}");
                        continue;
                    }
                }
            }
            "jumplists" => {
                let artifact = match &artifacts.jumplists {
                    Some(result) => result,
                    None => continue,
                };
                let results = jumplists(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Jumplists"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse jumplists, error: {err:?}");
                        continue;
                    }
                }
            }
            _ => warn!(
                "[artemis-core] Unsupported Windows artifact: {}",
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
    use super::windows_collection;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_windows_collection() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/quick.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        windows_collection(&buffer).unwrap();
    }
}
