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

        let filter = artifacts.filter.unwrap_or(false);
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
            #[cfg(feature = "boa")]
            "script" => {
                let script_data = &artifacts.script;
                let script = match script_data {
                    Some(result) => result,
                    _ => continue,
                };
                // Use the more descriptive script name as our artifact name
                artifacts.artifact_name = script.name.clone();
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
                let results = prefetch(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected prefetch"),
                    Err(err) => {
                        error!("[forensics] Failed to parse prefetch: {err:?}");
                    }
                }
            }
            "eventlogs" => {
                let options = match &artifacts.eventlogs {
                    Some(result) => result,
                    None => continue,
                };
                let results = eventlogs(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected Eventlogs"),
                    Err(err) => {
                        error!("[forensics] Failed to parse Eventlogs: {err:?}");
                    }
                }
            }
            "rawfiles" => {
                let options = match &artifacts.rawfiles {
                    Some(result) => result,
                    None => continue,
                };
                let results = raw_filelist(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected Raw Filelisting"),
                    Err(err) => {
                        error!("[forensics] Failed to get raw filelisting: {err:?}");
                    }
                }
            }
            "shimdb" => {
                let options = match &artifacts.shimdb {
                    Some(result) => result,
                    None => continue,
                };
                let results = shimdb(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected shimdb"),
                    Err(err) => {
                        error!("[forensics] Failed to parse shimdb: {err:?}");
                    }
                }
            }
            "registry" => {
                let options = match &artifacts.registry {
                    Some(result) => result,
                    None => continue,
                };
                let results = registry(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected registry"),
                    Err(err) => {
                        error!("[forensics] Failed to parse registry: {err:?}");
                    }
                }
            }
            "userassist" => {
                let options = match &artifacts.userassist {
                    Some(result) => result,
                    None => continue,
                };
                let results = userassist(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected userassist"),
                    Err(err) => {
                        error!("[forensics] Failed to parse userassist: {err:?}");
                    }
                }
            }
            "shimcache" => {
                let options = match &artifacts.shimcache {
                    Some(result) => result,
                    None => continue,
                };
                let results = shimcache(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected shimcache"),
                    Err(err) => {
                        error!("[forensics] Failed to parse shimcache: {err:?}");
                    }
                }
            }
            "shellbags" => {
                let options = match &artifacts.shellbags {
                    Some(result) => result,
                    None => continue,
                };
                let results = shellbags(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected shellbags"),
                    Err(err) => {
                        error!("[forensics] Failed to parse shellbags: {err:?}");
                    }
                }
            }
            "amcache" => {
                let options = match &artifacts.amcache {
                    Some(result) => result,
                    None => continue,
                };
                let results = amcache(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected amcache"),
                    Err(err) => {
                        error!("[forensics] Failed to parse amcache: {err:?}");
                    }
                }
            }
            "shortcuts" => {
                let options = match &artifacts.shortcuts {
                    Some(result) => result,
                    None => continue,
                };
                let results = shortcuts(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected shortcuts"),
                    Err(err) => {
                        error!("[forensics] Failed to parse shortcut files: {err:?}");
                    }
                }
            }
            "usnjrnl" => {
                let options = match &artifacts.usnjrnl {
                    Some(result) => result,
                    None => continue,
                };
                let results = usnjrnl(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected usnjrnl"),
                    Err(err) => {
                        error!("[forensics] Failed to parse usnjrnl: {err:?}");
                    }
                }
            }
            "bits" => {
                let options = match &artifacts.bits {
                    Some(result) => result,
                    None => continue,
                };
                let results = bits(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected bits"),
                    Err(err) => {
                        error!("[forensics] Failed to parse bits: {err:?}");
                    }
                }
            }
            "srum" => {
                let options = match &artifacts.srum {
                    Some(result) => result,
                    None => continue,
                };
                let results = srum(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected SRUM"),
                    Err(err) => {
                        error!("[forensics] Failed to parse srum: {err:?}");
                    }
                }
            }
            "search" => {
                let options = match &artifacts.search {
                    Some(result) => result,
                    None => continue,
                };
                let results = search(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected search"),
                    Err(err) => {
                        error!("[forensics] Failed to parse search: {err:?}");
                    }
                }
            }
            "users-windows" => {
                let options = match &artifacts.users_windows {
                    Some(result) => result,
                    None => continue,
                };
                let results = users_windows(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected Users"),
                    Err(err) => {
                        error!("[forensics] Failed to parse users: {err:?}");
                    }
                }
            }
            "tasks" => {
                let options = match &artifacts.tasks {
                    Some(result) => result,
                    None => continue,
                };
                let results = tasks(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected Schedule Tasks"),
                    Err(err) => {
                        error!("[forensics] Failed to parse schedule tasks: {err:?}");
                    }
                }
            }
            "services" => {
                let options = match &artifacts.services {
                    Some(result) => result,
                    None => continue,
                };
                let results = services(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected Services"),
                    Err(err) => {
                        error!("[forensics] Failed to parse services: {err:?}");
                    }
                }
            }
            "jumplists" => {
                let options = match &artifacts.jumplists {
                    Some(result) => result,
                    None => continue,
                };
                let results = jumplists(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected Jumplists"),
                    Err(err) => {
                        error!("[forensics] Failed to parse jumplists: {err:?}");
                    }
                }
            }
            "recyclebin" => {
                let options = match &artifacts.recyclebin {
                    Some(result) => result,
                    None => continue,
                };
                let results = recycle_bin(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected Recycle Bin"),
                    Err(err) => {
                        error!("[forensics] Failed to parse recycle bin: {err:?}");
                    }
                }
            }
            "wmipersist" => {
                let options = match &artifacts.wmipersist {
                    Some(result) => result,
                    None => continue,
                };
                let results = wmi_persist(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected WMI Persistence"),
                    Err(err) => {
                        error!("[forensics] Failed to parse WMI persistence: {err:?}");
                    }
                }
            }
            "outlook" => {
                let options = match &artifacts.outlook {
                    Some(result) => result,
                    None => continue,
                };
                let results = outlook(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected outlook"),
                    Err(err) => {
                        error!("[forensics] Failed to parse outlook: {err:?}");
                    }
                }
            }
            "mft" => {
                let options = match &artifacts.mft {
                    Some(result) => result,
                    None => continue,
                };
                let results = mft(options, &mut collector.output, filter);
                match results {
                    Ok(_) => info!("Collected MFT"),
                    Err(err) => {
                        error!("[forensics] Failed to parse MFT: {err:?}");
                    }
                }
            }
            "connections" => {
                let results = list_connections(&mut manager);
                match results {
                    Ok(_) => info!("Collected connections"),
                    Err(err) => {
                        error!("[forensics] Failed to parse MFT: {err:?}");
                        manager.write_failed_artifact(artifact, &"");
                    }
                }
            }
            "triage" => {
                let options = match &artifacts.triage {
                    Some(result) => result,
                    None => continue,
                };
                let results = triage(&mut collector.output, options);
                match results {
                    Ok(_) => info!("Collected connections"),
                    Err(err) => {
                        error!("[forensics] Failed to collect triage: {err:?}");
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
