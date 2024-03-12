use super::{
    applications::artifacts::{
        chromium_downloads, chromium_history, firefox_downloads, firefox_history, safari_downloads,
        safari_history,
    },
    os::{
        linux::artifacts::{journals, logons, sudo_logs_linux},
        macos::{
            artifacts::{
                emond, execpolicy, files, fseventsd, groups_macos, launchd, loginitems, processes,
                spotlight, sudo_logs_macos, systeminfo, unifiedlogs, users_macos,
            },
            error::MacArtifactError,
        },
        unix::artifacts::{bash_history, cron_job, python_history, zsh_history},
        windows::artifacts::{
            amcache, bits, eventlogs, jumplists, prefetch, raw_filelist, recycle_bin, registry,
            search, services, shellbags, shimcache, shimdb, shortcuts, srum, tasks, userassist,
            users_windows, usnjrnl, wmi_persist,
        },
    },
};
use crate::{
    runtime::deno::execute_script,
    structs::toml::ArtemisToml,
    utils::{logging::upload_logs, output::compress_final_output},
};
use log::{error, info, warn};

/// Parse the TOML collector and get artifacts
pub(crate) fn collect(collector: &mut ArtemisToml) -> Result<(), MacArtifactError> {
    // Loop through all supported artifacts
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
            "users-macos" => {
                let options = match &artifacts.users_macos {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = users_macos(&mut collector.output, &filter, options);
                match results {
                    Ok(_) => info!("Collected users"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse users, error: {err:?}");
                        continue;
                    }
                }
            }
            "groups-macos" => {
                let options = match &artifacts.groups_macos {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = groups_macos(&mut collector.output, &filter, options);
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
                    Ok(_) => info!("Collected bash history"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse bash history, error: {err:?}");
                        continue;
                    }
                }
                let results = zsh_history(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected zsh history"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse zsh history, error: {err:?}");
                        continue;
                    }
                }
                let results = python_history(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected python history"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse python history, error: {err:?}");
                        continue;
                    }
                }
            }
            "cron" => {
                let results = cron_job(&mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected cron"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse cron data, error: {err:?}");
                        continue;
                    }
                }
            }
            "sudologs-macos" => {
                let options = match &artifacts.sudologs_macos {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = sudo_logs_macos(&mut collector.output, &filter, options);
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
            "spotlight" => {
                let options = match &artifacts.spotlight {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = spotlight(&mut collector.output, &filter, options);
                match results {
                    Ok(_) => info!("Collected spotlight"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse spotlight, error: {err:?}");
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
            // Linux
            "journal" => {
                let options = match &artifacts.journals {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = journals(&mut collector.output, &filter, options);
                match results {
                    Ok(_) => info!("Collected journals"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse journals, error: {err:?}");
                        continue;
                    }
                }
            }
            "logon" => {
                let options = match &artifacts.logons {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = logons(&mut collector.output, &filter, options);
                match results {
                    Ok(_) => info!("Collected logons"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse logons, error: {err:?}");
                        continue;
                    }
                }
            }
            "sudologs-linux" => {
                let options = match &artifacts.sudologs_linux {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = sudo_logs_linux(&mut collector.output, &filter, options);
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
            // Windows
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
            "users-windows" => {
                let artifact = match &artifacts.users_windows {
                    Some(result) => result,
                    None => continue,
                };
                let results = users_windows(artifact, &mut collector.output, &filter);
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
            "recyclebin" => {
                let artifact = match &artifacts.recyclebin {
                    Some(result) => result,
                    None => continue,
                };
                let results = recycle_bin(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected Recycle Bin"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse recycle bin, error: {err:?}");
                        continue;
                    }
                }
            }
            "wmipersist" => {
                let artifact = match &artifacts.wmipersist {
                    Some(result) => result,
                    None => continue,
                };
                let results = wmi_persist(artifact, &mut collector.output, &filter);
                match results {
                    Ok(_) => info!("Collected WMI Persistence"),
                    Err(err) => {
                        error!("[artemis-core] Failed to parse WMI persistence, error: {err:?}");
                        continue;
                    }
                }
            }
            _ => warn!(
                "[artemis-core] Unsupported artifact: {}",
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
    use super::collect;
    use crate::{filesystem::files::read_file, structs::toml::ArtemisToml};
    use std::path::PathBuf;

    #[test]
    fn test_collect() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/quick.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let mut collector = ArtemisToml::parse_artemis_toml(&buffer).unwrap();
        collect(&mut collector).unwrap();
    }

    #[test]
    fn test_windows_collect() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/quick.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let mut collector = ArtemisToml::parse_artemis_toml(&buffer).unwrap();
        collect(&mut collector).unwrap();
    }

    #[test]
    fn test_linux_collect() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/quick.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let mut collector = ArtemisToml::parse_artemis_toml(&buffer).unwrap();
        collect(&mut collector).unwrap();
    }
}
