use super::{
    error::CollectionError,
    os::{
        connections::artifact::list_connections,
        files::artifact::filelisting,
        linux::artifacts::{journals, logons, sudo_logs_linux},
        macos::artifacts::{
            emond, execpolicy, fseventsd, groups_macos, launchd, loginitems, spotlight,
            sudo_logs_macos, unifiedlogs, users_macos,
        },
        processes::artifact::processes,
        systeminfo::artifact::systeminfo,
        windows::artifacts::{
            amcache, bits, eventlogs, jumplists, mft, outlook, prefetch, raw_filelist, recycle_bin,
            registry, search, services, shellbags, shimcache, shimdb, shortcuts, srum, tasks,
            userassist, users_windows, usnjrnl, wmi_persist,
        },
    },
};
use crate::{
    artifacts::os::linux::artifacts::ext4_filelist,
    runtime::run::execute_script,
    structs::toml::ArtemisToml,
    utils::{
        logging::upload_logs,
        marker::{skip_artifact, update_marker},
        output::compress_final_output,
    },
};
use log::{error, info, warn};

/// Parse the TOML collector and get artifacts
pub(crate) fn collect(collector: &mut ArtemisToml) -> Result<(), CollectionError> {
    // Loop through all supported artifacts
    for artifacts in &collector.artifacts {
        // If marker file is enabled, check if we should skip this artifact
        if collector.marker.is_some()
            && skip_artifact(collector.marker.as_ref().unwrap(), artifacts)
        {
            continue;
        }
        let filter = artifacts.filter.unwrap_or(false);
        match artifacts.artifact_name.as_str() {
            "loginitems" => {
                let options = match &artifacts.loginitems {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = loginitems(&mut collector.output, filter, options);
                match results {
                    Ok(_) => info!("Collected loginitems"),
                    Err(err) => {
                        error!("[forensics] Failed to parse loginitems: {err:?}");
                    }
                }
            }
            "emond" => {
                let options = match &artifacts.emond {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = emond(&mut collector.output, filter, options);
                match results {
                    Ok(_) => info!("Collected emond"),
                    Err(err) => {
                        error!("[forensics] Failed to parse emond: {err:?}");
                    }
                }
            }
            "fseventsd" => {
                let options = match &artifacts.fseventsd {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = fseventsd(&mut collector.output, filter, options);
                match results {
                    Ok(_) => info!("Collected fseventsd"),
                    Err(err) => {
                        error!("[forensics] Failed to parse fseventsd: {err:?}");
                    }
                }
            }
            "launchd" => {
                let options = match &artifacts.launchd {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = launchd(&mut collector.output, filter, options);
                match results {
                    Ok(_) => info!("Collected launchd"),
                    Err(err) => {
                        error!("[forensics] Failed to parse launchd: {err:?}");
                    }
                }
            }
            "files" => {
                let options = match &artifacts.files {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = filelisting(&mut collector.output, filter, options);
                match results {
                    Ok(_) => info!("Collected file listing"),
                    Err(err) => {
                        error!("[forensics] Failed to parse filelisting: {err:?}");
                    }
                }
            }
            "users-macos" => {
                let options = match &artifacts.users_macos {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = users_macos(&mut collector.output, filter, options);
                match results {
                    Ok(_) => info!("Collected users"),
                    Err(err) => {
                        error!("[forensics] Failed to parse users: {err:?}");
                    }
                }
            }
            "groups-macos" => {
                let options = match &artifacts.groups_macos {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = groups_macos(&mut collector.output, filter, options);
                match results {
                    Ok(_) => info!("Collected groups"),
                    Err(err) => {
                        error!("[forensics] Failed to parse groups: {err:?}");
                    }
                }
            }
            "processes" => {
                let options = match &artifacts.processes {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = processes(&mut collector.output, filter, options);
                match results {
                    Ok(_) => info!("Collected processes"),
                    Err(err) => {
                        error!("[forensics] Failed to parse processes: {err:?}");
                    }
                }
            }
            "systeminfo" => {
                let results = systeminfo(&mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected systeminfo"),
                    Err(err) => {
                        error!("[forensics] Failed to parse systeminfo: {err:?}");
                    }
                }
            }
            "execpolicy" => {
                let options = match &artifacts.execpolicy {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = execpolicy(&mut collector.output, filter, options);
                match results {
                    Ok(_) => info!("Collected execpolicy"),
                    Err(err) => {
                        error!("[forensics] Failed to parse execpolicy: {err:?}");
                    }
                }
            }
            "unifiedlogs" => {
                let options = match &artifacts.unifiedlogs {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = unifiedlogs(&mut collector.output, filter, options);
                match results {
                    Ok(_) => info!("Collected unified logs"),
                    Err(err) => {
                        error!("[forensics] Failed to parse unified logs: {err:?}");
                    }
                }
            }
            "sudologs-macos" => {
                let options = match &artifacts.sudologs_macos {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = sudo_logs_macos(&mut collector.output, filter, options);
                match results {
                    Ok(_) => info!("Collected macOS sudo logs"),
                    Err(err) => {
                        error!("[forensics] Failed to parse macOS sudo log data: {err:?}");
                    }
                }
            }
            "spotlight" => {
                let options = match &artifacts.spotlight {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = spotlight(&mut collector.output, filter, options);
                match results {
                    Ok(_) => info!("Collected spotlight"),
                    Err(err) => {
                        error!("[forensics] Failed to parse spotlight: {err:?}");
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
                        error!("[forensics] Failed to execute JavaScript error: {err:?}");
                    }
                }
            }
            // Linux
            "journal" => {
                let options = match &artifacts.journals {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = journals(&mut collector.output, filter, options);
                match results {
                    Ok(_) => info!("Collected journals"),
                    Err(err) => {
                        error!("[forensics] Failed to parse journals: {err:?}");
                    }
                }
            }
            "logons" => {
                let options = match &artifacts.logons {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = logons(&mut collector.output, filter, options);
                match results {
                    Ok(_) => info!("Collected logons"),
                    Err(err) => {
                        error!("[forensics] Failed to parse logons: {err:?}");
                    }
                }
            }
            "sudologs-linux" => {
                let options = match &artifacts.sudologs_linux {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = sudo_logs_linux(&mut collector.output, filter, options);
                match results {
                    Ok(_) => info!("Collected Linux sudo logs"),
                    Err(err) => {
                        error!("[forensics] Failed to parse Linux sudo log data: {err:?}");
                    }
                }
            }
            "rawfiles-ext4" => {
                let options = match &artifacts.rawfiles_ext4 {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = ext4_filelist(&mut collector.output, filter, options);
                match results {
                    Ok(_) => info!("Collected Linux raw ext4 file listing"),
                    Err(err) => {
                        error!("[forensics] Failed to parse Linux ext4 filesystem: {err:?}");
                    }
                }
            }
            // Windows
            "prefetch" => {
                let artifact = match &artifacts.prefetch {
                    Some(result) => result,
                    None => continue,
                };
                let results = prefetch(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected prefetch"),
                    Err(err) => {
                        error!("[forensics] Failed to parse prefetch: {err:?}");
                    }
                }
            }
            "eventlogs" => {
                let artifact = match &artifacts.eventlogs {
                    Some(result) => result,
                    None => continue,
                };
                let results = eventlogs(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected Eventlogs"),
                    Err(err) => {
                        error!("[forensics] Failed to parse Eventlogs: {err:?}");
                    }
                }
            }
            "rawfiles" => {
                let artifact = match &artifacts.rawfiles {
                    Some(result) => result,
                    None => continue,
                };
                let results = raw_filelist(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected Raw Filelisting"),
                    Err(err) => {
                        error!("[forensics] Failed to get raw filelisting: {err:?}");
                    }
                }
            }
            "shimdb" => {
                let artifact = match &artifacts.shimdb {
                    Some(result) => result,
                    None => continue,
                };
                let results = shimdb(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected shimdb"),
                    Err(err) => {
                        error!("[forensics] Failed to parse shimdb: {err:?}");
                    }
                }
            }
            "registry" => {
                let artifact = match &artifacts.registry {
                    Some(result) => result,
                    None => continue,
                };
                let results = registry(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected registry"),
                    Err(err) => {
                        error!("[forensics] Failed to parse registry: {err:?}");
                    }
                }
            }
            "userassist" => {
                let artifact = match &artifacts.userassist {
                    Some(result) => result,
                    None => continue,
                };
                let results = userassist(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected userassist"),
                    Err(err) => {
                        error!("[forensics] Failed to parse userassist: {err:?}");
                    }
                }
            }
            "shimcache" => {
                let artifact = match &artifacts.shimcache {
                    Some(result) => result,
                    None => continue,
                };
                let results = shimcache(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected shimcache"),
                    Err(err) => {
                        error!("[forensics] Failed to parse shimcache: {err:?}");
                    }
                }
            }
            "shellbags" => {
                let artifact = match &artifacts.shellbags {
                    Some(result) => result,
                    None => continue,
                };
                let results = shellbags(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected shellbags"),
                    Err(err) => {
                        error!("[forensics] Failed to parse shellbags: {err:?}");
                    }
                }
            }
            "amcache" => {
                let artifact = match &artifacts.amcache {
                    Some(result) => result,
                    None => continue,
                };
                let results = amcache(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected amcache"),
                    Err(err) => {
                        error!("[forensics] Failed to parse amcache: {err:?}");
                    }
                }
            }
            "shortcuts" => {
                let artifact = match &artifacts.shortcuts {
                    Some(result) => result,
                    None => continue,
                };
                let results = shortcuts(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected shortcuts"),
                    Err(err) => {
                        error!("[forensics] Failed to parse shortcut files: {err:?}");
                    }
                }
            }
            "usnjrnl" => {
                let artifact = match &artifacts.usnjrnl {
                    Some(result) => result,
                    None => continue,
                };
                let results = usnjrnl(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected usnjrnl"),
                    Err(err) => {
                        error!("[forensics] Failed to parse usnjrnl: {err:?}");
                    }
                }
            }
            "bits" => {
                let artifact = match &artifacts.bits {
                    Some(result) => result,
                    None => continue,
                };
                let results = bits(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected bits"),
                    Err(err) => {
                        error!("[forensics] Failed to parse bits: {err:?}");
                    }
                }
            }
            "srum" => {
                let artifact = match &artifacts.srum {
                    Some(result) => result,
                    None => continue,
                };
                let results = srum(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected SRUM"),
                    Err(err) => {
                        error!("[forensics] Failed to parse srum: {err:?}");
                    }
                }
            }
            "search" => {
                let artifact = match &artifacts.search {
                    Some(result) => result,
                    None => continue,
                };
                let results = search(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected search"),
                    Err(err) => {
                        error!("[forensics] Failed to parse search: {err:?}");
                    }
                }
            }
            "users-windows" => {
                let artifact = match &artifacts.users_windows {
                    Some(result) => result,
                    None => continue,
                };
                let results = users_windows(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected Users"),
                    Err(err) => {
                        error!("[forensics] Failed to parse users: {err:?}");
                    }
                }
            }
            "tasks" => {
                let artifact = match &artifacts.tasks {
                    Some(result) => result,
                    None => continue,
                };
                let results = tasks(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected Schedule Tasks"),
                    Err(err) => {
                        error!("[forensics] Failed to parse schedule tasks: {err:?}");
                    }
                }
            }
            "services" => {
                let artifact = match &artifacts.services {
                    Some(result) => result,
                    None => continue,
                };
                let results = services(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected Services"),
                    Err(err) => {
                        error!("[forensics] Failed to parse services: {err:?}");
                    }
                }
            }
            "jumplists" => {
                let artifact = match &artifacts.jumplists {
                    Some(result) => result,
                    None => continue,
                };
                let results = jumplists(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected Jumplists"),
                    Err(err) => {
                        error!("[forensics] Failed to parse jumplists: {err:?}");
                    }
                }
            }
            "recyclebin" => {
                let artifact = match &artifacts.recyclebin {
                    Some(result) => result,
                    None => continue,
                };
                let results = recycle_bin(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected Recycle Bin"),
                    Err(err) => {
                        error!("[forensics] Failed to parse recycle bin: {err:?}");
                    }
                }
            }
            "wmipersist" => {
                let artifact = match &artifacts.wmipersist {
                    Some(result) => result,
                    None => continue,
                };
                let results = wmi_persist(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected WMI Persistence"),
                    Err(err) => {
                        error!("[forensics] Failed to parse WMI persistence: {err:?}");
                    }
                }
            }
            "outlook" => {
                let artifact = match &artifacts.outlook {
                    Some(result) => result,
                    None => continue,
                };
                let results = outlook(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected outlook"),
                    Err(err) => {
                        error!("[forensics] Failed to parse outlook: {err:?}");
                    }
                }
            }
            "mft" => {
                let artifact = match &artifacts.mft {
                    Some(result) => result,
                    None => continue,
                };
                let results = mft(artifact, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected MFT"),
                    Err(err) => {
                        error!("[forensics] Failed to parse MFT: {err:?}");
                    }
                }
            }
            "connections" => {
                let results = list_connections(&mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected connections"),
                    Err(err) => {
                        error!("[forensics] Failed to parse MFT: {err:?}");
                    }
                }
            }
            _ => warn!(
                "[forensics] Unsupported artifact: {}",
                artifacts.artifact_name
            ),
        }

        // If marker file is enabled, write and update a marker file to track most recent artifact runs
        if collector.marker.is_some() {
            update_marker(collector.marker.as_ref().unwrap(), artifacts);
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
    fn test_macos_collect() {
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

    #[test]
    fn test_marker_collect() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/marker.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let mut collector = ArtemisToml::parse_artemis_toml(&buffer).unwrap();
        collect(&mut collector).unwrap();
    }
}
