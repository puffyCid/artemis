use super::{
    error::CollectionError,
    os::{
        connections::artifact::list_connections,
        files::artifact::filelisting,
        linux::artifacts::{ext4_filelist, journals, logons, sudo_logs_linux},
        macos::artifacts::{
            emond, execpolicy, fseventsd, groups_macos, launchd, loginitems, spotlight,
            sudo_logs_macos, unifiedlogs, users_macos,
        },
        processes::artifact::processes,
        systeminfo::artifact::systeminfo,
        triage::artifact::triage,
        windows::artifacts::{
            amcache, bits, eventlogs, jumplists, mft, outlook, prefetch, raw_filelist, recycle_bin,
            registry, search, services, shellbags, shimcache, shimdb, shortcuts, srum, tasks,
            userassist, users_windows, usnjrnl, wmi_persist,
        },
    },
};
use crate::{
    output2::{config::OutputConfig, manager::OutputManager},
    structs::toml::ArtemisToml,
    utils::marker::skip_artifact,
};
use log::{error, info, warn};

#[cfg(feature = "boa")]
use crate::runtime::run::execute_script;

/// Parse the TOML collector and get artifacts
pub(crate) fn collect(mut collector: ArtemisToml) -> Result<(), CollectionError> {
    let config = match OutputConfig::try_from(collector.output.clone()) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Could not setup output config: {err:?}");
            return Err(CollectionError::Output);
        }
    };
    let mut manager = match OutputManager::new(config) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Could not setup output manager: {err:?}");
            return Err(CollectionError::Output);
        }
    };

    // Loop through all supported artifacts
    for artifacts in &mut collector.artifacts {
        // If marker file is enabled, check if we should skip this artifact
        if collector.marker.is_some()
            && skip_artifact(collector.marker.as_ref().unwrap(), artifacts)
        {
            continue;
        }

        //let filter = artifacts.filter.unwrap_or(false);
        let artifact = artifacts.artifact_name.as_str();
        match artifact {
            "loginitems" => {
                let options = match &artifacts.loginitems {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = loginitems(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected loginitems"),
                    Err(err) => {
                        error!("[forensics] Failed to parse loginitems: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "emond" => {
                let options = match &artifacts.emond {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = emond(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected emond"),
                    Err(err) => {
                        error!("[forensics] Failed to parse emond: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "fseventsd" => {
                let options = match &artifacts.fseventsd {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = fseventsd(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected fseventsd"),
                    Err(err) => {
                        error!("[forensics] Failed to parse fseventsd: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "launchd" => {
                let options = match &artifacts.launchd {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = launchd(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected launchd"),
                    Err(err) => {
                        error!("[forensics] Failed to parse launchd: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "files" => {
                let options = match &artifacts.files {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = filelisting(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected file listing"),
                    Err(err) => {
                        error!("[forensics] Failed to parse filelisting: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "users-macos" => {
                let options = match &artifacts.users_macos {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = users_macos(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected users"),
                    Err(err) => {
                        error!("[forensics] Failed to parse users: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "groups-macos" => {
                let options = match &artifacts.groups_macos {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = groups_macos(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected groups"),
                    Err(err) => {
                        error!("[forensics] Failed to parse groups: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "processes" => {
                let options = match &artifacts.processes {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = processes(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected processes"),
                    Err(err) => {
                        error!("[forensics] Failed to parse processes: {err:?}");
                        manager.write_failed_artifact(artifact, &options);
                    }
                }
            }
            "systeminfo" => {
                let results = systeminfo(&mut manager);
                match results {
                    Ok(_) => info!("Collected systeminfo"),
                    Err(err) => {
                        error!("[forensics] Failed to parse systeminfo: {err:?}");
                        manager.write_failed_artifact(artifact, &"");
                    }
                }
            }
            "execpolicy" => {
                let options = match &artifacts.execpolicy {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = execpolicy(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected execpolicy"),
                    Err(err) => {
                        error!("[forensics] Failed to parse execpolicy: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "unifiedlogs" => {
                let options = match &artifacts.unifiedlogs {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = unifiedlogs(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected unified logs"),
                    Err(err) => {
                        error!("[forensics] Failed to parse unified logs: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "sudologs-macos" => {
                let options = match &artifacts.sudologs_macos {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = sudo_logs_macos(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected macOS sudo logs"),
                    Err(err) => {
                        error!("[forensics] Failed to parse macOS sudo log data: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "spotlight" => {
                let options = match &artifacts.spotlight {
                    Some(result_data) => result_data,
                    _ => continue,
                };
                let results = spotlight(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected spotlight"),
                    Err(err) => {
                        error!("[forensics] Failed to parse spotlight: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            #[cfg(feature = "boa")]
            "script" => {
                let script_data = &artifacts.script;
                let script = match script_data {
                    Some(result) => result,
                    _ => continue,
                };
                // Use the more descriptive script name as our artifact name
                artifacts.artifact_name = script.name.clone();
                let results = execute_script(&mut manager, script);
                match results {
                    Ok(_) => info!("Executed JavaScript "),
                    Err(err) => {
                        error!("[forensics] Failed to execute JavaScript error: {err:?}");
                    }
                }
            }
            // Linux
            "journal" => {
                let options = match &artifacts.journal {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = journals(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected journals"),
                    Err(err) => {
                        error!("[forensics] Failed to parse journals: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "logons" => {
                let options = match &artifacts.logons {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = logons(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected logons"),
                    Err(err) => {
                        error!("[forensics] Failed to parse logons: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "sudologs-linux" => {
                let options = match &artifacts.sudologs_linux {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = sudo_logs_linux(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected Linux sudo logs"),
                    Err(err) => {
                        error!("[forensics] Failed to parse Linux sudo log data: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "rawfiles-ext4" => {
                let options = match &artifacts.rawfiles_ext4 {
                    Some(result_data) => result_data,
                    _ => continue,
                };

                let results = ext4_filelist(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected Linux raw ext4 file listing"),
                    Err(err) => {
                        error!("[forensics] Failed to parse Linux ext4 filesystem: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            // Windows
            "prefetch" => {
                let options = match &artifacts.prefetch {
                    Some(result) => result,
                    None => continue,
                };
                let results = prefetch(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected prefetch"),
                    Err(err) => {
                        error!("[forensics] Failed to parse prefetch: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "eventlogs" => {
                let options = match &artifacts.eventlogs {
                    Some(result) => result,
                    None => continue,
                };
                let results = eventlogs(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected Eventlogs"),
                    Err(err) => {
                        error!("[forensics] Failed to parse Eventlogs: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "rawfiles" => {
                let options = match &artifacts.rawfiles {
                    Some(result) => result,
                    None => continue,
                };
                let results = raw_filelist(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected Raw Filelisting"),
                    Err(err) => {
                        error!("[forensics] Failed to get raw filelisting: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "shimdb" => {
                let options = match &artifacts.shimdb {
                    Some(result) => result,
                    None => continue,
                };
                let results = shimdb(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected shimdb"),
                    Err(err) => {
                        error!("[forensics] Failed to parse shimdb: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "registry" => {
                let options = match &artifacts.registry {
                    Some(result) => result,
                    None => continue,
                };
                let results = registry(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected registry"),
                    Err(err) => {
                        error!("[forensics] Failed to parse registry: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "userassist" => {
                let options = match &artifacts.userassist {
                    Some(result) => result,
                    None => continue,
                };
                let results = userassist(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected userassist"),
                    Err(err) => {
                        error!("[forensics] Failed to parse userassist: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "shimcache" => {
                let options = match &artifacts.shimcache {
                    Some(result) => result,
                    None => continue,
                };
                let results = shimcache(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected shimcache"),
                    Err(err) => {
                        error!("[forensics] Failed to parse shimcache: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "shellbags" => {
                let options = match &artifacts.shellbags {
                    Some(result) => result,
                    None => continue,
                };
                let results = shellbags(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected shellbags"),
                    Err(err) => {
                        error!("[forensics] Failed to parse shellbags: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "amcache" => {
                let options = match &artifacts.amcache {
                    Some(result) => result,
                    None => continue,
                };
                let results = amcache(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected amcache"),
                    Err(err) => {
                        error!("[forensics] Failed to parse amcache: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "shortcuts" => {
                let options = match &artifacts.shortcuts {
                    Some(result) => result,
                    None => continue,
                };
                let results = shortcuts(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected shortcuts"),
                    Err(err) => {
                        error!("[forensics] Failed to parse shortcut files: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "usnjrnl" => {
                let options = match &artifacts.usnjrnl {
                    Some(result) => result,
                    None => continue,
                };
                let results = usnjrnl(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected usnjrnl"),
                    Err(err) => {
                        error!("[forensics] Failed to parse usnjrnl: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "bits" => {
                let options = match &artifacts.bits {
                    Some(result) => result,
                    None => continue,
                };
                let results = bits(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected bits"),
                    Err(err) => {
                        error!("[forensics] Failed to parse bits: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "srum" => {
                let options = match &artifacts.srum {
                    Some(result) => result,
                    None => continue,
                };
                let results = srum(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected SRUM"),
                    Err(err) => {
                        error!("[forensics] Failed to parse srum: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "search" => {
                let options = match &artifacts.search {
                    Some(result) => result,
                    None => continue,
                };
                let results = search(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected search"),
                    Err(err) => {
                        error!("[forensics] Failed to parse search: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "users-windows" => {
                let options = match &artifacts.users_windows {
                    Some(result) => result,
                    None => continue,
                };
                let results = users_windows(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected Users"),
                    Err(err) => {
                        error!("[forensics] Failed to parse users: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "tasks" => {
                let options = match &artifacts.tasks {
                    Some(result) => result,
                    None => continue,
                };
                let results = tasks(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected Schedule Tasks"),
                    Err(err) => {
                        error!("[forensics] Failed to parse schedule tasks: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "services" => {
                let options = match &artifacts.services {
                    Some(result) => result,
                    None => continue,
                };
                let results = services(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected Services"),
                    Err(err) => {
                        error!("[forensics] Failed to parse services: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "jumplists" => {
                let options = match &artifacts.jumplists {
                    Some(result) => result,
                    None => continue,
                };
                let results = jumplists(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected Jumplists"),
                    Err(err) => {
                        error!("[forensics] Failed to parse jumplists: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "recyclebin" => {
                let options = match &artifacts.recyclebin {
                    Some(result) => result,
                    None => continue,
                };
                let results = recycle_bin(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected Recycle Bin"),
                    Err(err) => {
                        error!("[forensics] Failed to parse recycle bin: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "wmipersist" => {
                let options = match &artifacts.wmipersist {
                    Some(result) => result,
                    None => continue,
                };
                let results = wmi_persist(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected WMI Persistence"),
                    Err(err) => {
                        error!("[forensics] Failed to parse WMI persistence: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "outlook" => {
                let options = match &artifacts.outlook {
                    Some(result) => result,
                    None => continue,
                };
                let results = outlook(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected outlook"),
                    Err(err) => {
                        error!("[forensics] Failed to parse outlook: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "mft" => {
                let options = match &artifacts.mft {
                    Some(result) => result,
                    None => continue,
                };
                let results = mft(options, &mut manager);
                match results {
                    Ok(_) => info!("Collected MFT"),
                    Err(err) => {
                        error!("[forensics] Failed to parse MFT: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            "connections" => {
                let results = list_connections(&mut manager);
                match results {
                    Ok(_) => info!("Collected connections"),
                    Err(err) => {
                        error!("[forensics] Failed to parse connections: {err:?}");
                        manager.write_failed_artifact(artifact, &"");
                    }
                }
            }
            "triage" => {
                let options = match &artifacts.triage {
                    Some(result) => result,
                    None => continue,
                };
                let results = triage(&mut manager, options);
                match results {
                    Ok(_) => info!("Collected connections"),
                    Err(err) => {
                        error!("[forensics] Failed to collect triage: {err:?}");
                        manager.write_failed_artifact(artifact, options);
                    }
                }
            }
            _ => warn!(
                "[forensics] Unsupported artifact: {}",
                artifacts.artifact_name
            ),
        }
    }

    if let Err(err) = manager.finalize() {
        error!("[forensics] Could not finalize collection: {err:?}");
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
        let collector = ArtemisToml::parse_artemis_toml(&buffer).unwrap();
        collect(collector).unwrap();
    }

    #[test]
    fn test_windows_collect() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/quick.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let collector = ArtemisToml::parse_artemis_toml(&buffer).unwrap();
        collect(collector).unwrap();
    }

    #[test]
    fn test_linux_collect() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/linux/quick.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let collector = ArtemisToml::parse_artemis_toml(&buffer).unwrap();
        collect(collector).unwrap();
    }

    #[test]
    fn test_marker_collect() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/marker.toml");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let collector = ArtemisToml::parse_artemis_toml(&buffer).unwrap();
        collect(collector).unwrap();
    }
}
