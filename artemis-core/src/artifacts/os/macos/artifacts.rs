use super::{
    accounts::{groups::grab_groups, users::grab_users},
    emond::parser::grab_emond,
    error::MacArtifactError,
    fsevents::parser::grab_fseventsd,
    launchd::launchdaemon::grab_launchd,
    loginitems::parser::grab_loginitems,
    unified_logs::logs::grab_logs,
};
use crate::{
    artifacts::os::{
        files::filelisting::FileInfo, processes::process::Processes, systeminfo::info::SystemInfo,
    },
    filesystem::files::Hashes,
    output::formats::{json::json_format, jsonl::jsonl_format},
    runtime::deno::filter_script,
    structs::artifacts::os::{files::FileOptions, processes::ProcessOptions},
    utils::{artemis_toml::Output, time},
};
use log::{error, warn};
use macos_unifiedlogs::parser::{
    collect_shared_strings_system, collect_strings_system, collect_timesync_system,
};
use serde_json::Value;

/// Parse macOS `LoginItems`
pub(crate) fn loginitems(output: &mut Output, filter: &bool) -> Result<(), MacArtifactError> {
    let start_time = time::time_now();

    let artifact_result = grab_loginitems();
    let result = match artifact_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Artemis macOS failed to parse loginitems: {err:?}");
            return Err(MacArtifactError::LoginItem);
        }
    };

    let serde_data_result = serde_json::to_value(result);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize loginitems: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let output_name = "loginitems";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Parse macOS `Emond`
pub(crate) fn emond(output: &mut Output, filter: &bool) -> Result<(), MacArtifactError> {
    let start_time = time::time_now();

    let results = grab_emond();
    let emond_data = match results {
        Ok(result) => result,
        Err(err) => {
            warn!("[artemis-core] Artemis macOS failed to parse emond rules: {err:?}");
            return Err(MacArtifactError::Emond);
        }
    };

    let serde_data_result = serde_json::to_value(emond_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize emond: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let output_name = "emond";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get macOS `Users`
pub(crate) fn users(output: &mut Output, filter: &bool) -> Result<(), MacArtifactError> {
    let start_time = time::time_now();

    let users_data = grab_users();
    let serde_data_result = serde_json::to_value(users_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize users: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let output_name = "users";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get macOS `Processes`
pub(crate) fn processes(
    artifact: &ProcessOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), MacArtifactError> {
    let start_time = time::time_now();

    let hashes = Hashes {
        md5: artifact.md5,
        sha1: artifact.sha1,
        sha256: artifact.sha256,
    };

    let results = Processes::proc_list(&hashes, artifact.metadata);
    let proc_data = match results {
        Ok(data) => data,
        Err(err) => {
            warn!("[artemis-core] Artemis macOS failed to get process list: {err:?}");
            return Err(MacArtifactError::Process);
        }
    };

    let serde_data_result = serde_json::to_value(proc_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize processes: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let output_name = "processes";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get macOS `Systeminfo`
pub(crate) fn systeminfo(output: &mut Output, filter: &bool) -> Result<(), MacArtifactError> {
    let start_time = time::time_now();

    let system_data = SystemInfo::get_info();
    let serde_data_result = serde_json::to_value(system_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize system data: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let output_name = "systeminfo";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get macOS `Groups`
pub(crate) fn groups(output: &mut Output, filter: &bool) -> Result<(), MacArtifactError> {
    let start_time = time::time_now();

    let groups_data = grab_groups();
    let serde_data_result = serde_json::to_value(groups_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize groups: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let output_name = "groups";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Parse macOS `FsEvents`
pub(crate) fn fseventsd(output: &mut Output, filter: &bool) -> Result<(), MacArtifactError> {
    let start_time = time::time_now();

    let artifact_result = grab_fseventsd();
    let results = match artifact_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Artemis macOS failed to parse fseventsd: {err:?}");
            return Err(MacArtifactError::FsEventsd);
        }
    };

    let serde_data_result = serde_json::to_value(results);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize fseventsd: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let output_name = "fseventsd";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Parse macOS `Launchd`
pub(crate) fn launchd(output: &mut Output, filter: &bool) -> Result<(), MacArtifactError> {
    let start_time = time::time_now();

    let artifact_result = grab_launchd();
    let results = match artifact_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Artemis macOS failed to parse launchd: {err:?}");
            return Err(MacArtifactError::Launchd);
        }
    };

    let serde_data_result = serde_json::to_value(results);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize launchd: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let output_name = "launchd";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get macOS `filelist`
pub(crate) fn files(
    artifact: &FileOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), MacArtifactError> {
    let hashes = Hashes {
        md5: artifact.md5.unwrap_or(false),
        sha1: artifact.sha1.unwrap_or(false),
        sha256: artifact.sha256.unwrap_or(false),
    };
    let artifact_result = FileInfo::get_filelist(
        &artifact.start_path,
        artifact.depth.unwrap_or(1).into(),
        artifact.metadata.unwrap_or(false),
        &hashes,
        artifact.regex_filter.as_ref().unwrap_or(&String::new()),
        output,
        filter,
    );
    match artifact_result {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Artemis macOS failed to get file listing: {err:?}");
            return Err(MacArtifactError::File);
        }
    };
    Ok(())
}

/// Get macOS `Unifiedlogs`
pub(crate) fn unifiedlogs(
    output: &mut Output,
    log_sources: &[String],
    filter: &bool,
) -> Result<(), MacArtifactError> {
    let start_time = time::time_now();

    // Need to first get the strings and timestamp data first before parsing the actual logs
    let strings_results = collect_strings_system();
    let shared_strings_results = collect_shared_strings_system();
    let timesync_data_results = collect_timesync_system();

    let strings = match strings_results {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to parse UUIDText files: {err:?}");
            return Err(MacArtifactError::UnifiedLogs);
        }
    };

    let shared_strings = match shared_strings_results {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to parse dsc files: {err:?}");
            return Err(MacArtifactError::UnifiedLogs);
        }
    };

    let timesync_data = match timesync_data_results {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to parse timesync files: {err:?}");
            return Err(MacArtifactError::UnifiedLogs);
        }
    };

    // Based on provided log sources provided in TOML file, parse the logs
    grab_logs(
        &strings,
        &shared_strings,
        &timesync_data,
        output,
        &start_time,
        log_sources,
        filter,
    )
}

/// Output macOS artifacts
pub(crate) fn output_data(
    serde_data: &Value,
    output_name: &str,
    output: &mut Output,
    start_time: &u64,
    filter: &bool,
) -> Result<(), MacArtifactError> {
    if *filter {
        if let Some(script) = &output.filter_script.clone() {
            let args = vec![serde_data.to_string(), output_name.to_string()];
            if let Some(name) = &output.filter_name.clone() {
                let filter_result = filter_script(output, &args, name, script);
                return match filter_result {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        error!(
                            "[artemis-core] Could not apply filter script to macos data: {err:?}"
                        );
                        Err(MacArtifactError::FilterOutput)
                    }
                };
            }
            let filter_result = filter_script(output, &args, "UnknownFilterName", script);
            return match filter_result {
                Ok(_) => Ok(()),
                Err(err) => {
                    error!(
                    "[artemis-core] Could not apply unknown filter script to macos data: {err:?}"
                );
                    Err(MacArtifactError::FilterOutput)
                }
            };
        }
    }

    let output_status = if output.format == "json" {
        json_format(serde_data, output_name, output, start_time)
    } else if output.format == "jsonl" {
        jsonl_format(serde_data, output_name, output, start_time)
    } else {
        error!(
            "[artemis-core] Unknown formatter provided: {}",
            output.format
        );
        return Err(MacArtifactError::Format);
    };
    match output_status {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Could not output data: {err:?}");
            return Err(MacArtifactError::Output);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::macos::artifacts::{
            emond, files, fseventsd, groups, launchd, loginitems, output_data, processes,
            systeminfo, unifiedlogs, users,
        },
        structs::artifacts::os::{files::FileOptions, processes::ProcessOptions},
        utils::{artemis_toml::Output, time},
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
        }
    }

    #[test]
    fn test_loginitems() {
        let mut output = output_options("loginitems_test", "local", "./tmp", false);

        let status = loginitems(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_emond() {
        let mut output = output_options("emond_test", "local", "./tmp", false);

        let status = emond(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_users() {
        let mut output = output_options("users_test", "local", "./tmp", false);

        let status = users(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_groups() {
        let mut output = output_options("groups_test", "local", "./tmp", false);

        let status = groups(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    #[ignore = "requires root"]
    fn test_fseventsd() {
        let mut output = output_options("fseventsd_test", "local", "./tmp", false);

        let status = fseventsd(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_launchd() {
        let mut output = output_options("launchd_test", "local", "./tmp", false);

        let status = launchd(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_processes() {
        let mut output = output_options("processes_test", "local", "./tmp", false);

        let proc_config = ProcessOptions {
            md5: true,
            sha1: true,
            sha256: true,
            metadata: true,
        };

        let status = processes(&proc_config, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_system() {
        let mut output = output_options("system_test", "local", "./tmp", false);

        let status = systeminfo(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_unifiedlogs() {
        let mut output = output_options("unifiedlogs_test", "local", "./tmp", false);
        let sources = vec![String::from("Special")];

        let status = unifiedlogs(&mut output, &sources, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_files() {
        let mut output = output_options("file_test", "local", "./tmp", false);

        let file_config = FileOptions {
            start_path: String::from("/"),
            depth: Some(1),
            metadata: Some(false),
            md5: Some(false),
            sha1: Some(false),
            sha256: Some(false),
            regex_filter: Some(String::new()),
        };
        let status = files(&file_config, &mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data() {
        let mut output = output_options("output_test", "local", "./tmp", false);
        let start_time = time::time_now();

        let name = "test";
        let data = serde_json::Value::String(String::from("test"));
        let status = output_data(&data, name, &mut output, &start_time, &&false).unwrap();
        assert_eq!(status, ());
    }
}
