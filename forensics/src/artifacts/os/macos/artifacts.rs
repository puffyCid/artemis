use super::{
    accounts::{groups::grab_groups, users::grab_users},
    emond::parser::grab_emond,
    error::MacArtifactError,
    execpolicy::policy::grab_execpolicy,
    fsevents::parser::grab_fseventsd,
    launchd::launchdaemon::grab_launchd,
    loginitems::parser::grab_loginitems,
    spotlight::parser::grab_spotlight,
    sudo::logs::grab_sudo_logs,
    unified_logs::logs::grab_logs,
};
use crate::{
    artifacts::output::output_artifact,
    structs::{
        artifacts::os::macos::{
            EmondOptions, ExecPolicyOptions, FseventsOptions, LaunchdOptions, LoginitemsOptions,
            MacosGroupsOptions, MacosSudoOptions, MacosUsersOptions, SpotlightOptions,
            UnifiedLogsOptions,
        },
        toml::Output,
    },
    utils::time::{self, time_now},
};
use log::{error, warn};
use serde_json::Value;

/// Parse macOS `LoginItems`
pub(crate) fn loginitems(
    output: &mut Output,
    filter: bool,
    options: &LoginitemsOptions,
) -> Result<(), MacArtifactError> {
    let start_time = time::time_now();

    let artifact_result = grab_loginitems(options);
    let result = match artifact_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to parse loginitems: {err:?}");
            return Err(MacArtifactError::LoginItem);
        }
    };

    let serde_data_result = serde_json::to_value(result);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize loginitems: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let output_name = "loginitems";
    output_data(&mut serde_data, output_name, output, &start_time, filter)
}

/// Parse macOS `Emond`
pub(crate) fn emond(
    output: &mut Output,
    filter: bool,
    options: &EmondOptions,
) -> Result<(), MacArtifactError> {
    let start_time = time::time_now();

    let results = grab_emond(options);
    let emond_data = match results {
        Ok(result) => result,
        Err(err) => {
            warn!("[core] Failed to parse emond rules: {err:?}");
            return Err(MacArtifactError::Emond);
        }
    };

    let serde_data_result = serde_json::to_value(emond_data);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize emond: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let output_name = "emond";
    output_data(&mut serde_data, output_name, output, &start_time, filter)
}

/// Get macOS `Users`
pub(crate) fn users_macos(
    output: &mut Output,
    filter: bool,
    options: &MacosUsersOptions,
) -> Result<(), MacArtifactError> {
    let start_time = time::time_now();

    let users_data = grab_users(options);
    let serde_data_result = serde_json::to_value(users_data);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize users: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let output_name = "users-macos";
    output_data(&mut serde_data, output_name, output, &start_time, filter)
}

/// Get macOS `Groups`
pub(crate) fn groups_macos(
    output: &mut Output,
    filter: bool,
    options: &MacosGroupsOptions,
) -> Result<(), MacArtifactError> {
    let start_time = time::time_now();

    let groups_data = grab_groups(options);
    let serde_data_result = serde_json::to_value(groups_data);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize groups: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let output_name = "groups-macos";
    output_data(&mut serde_data, output_name, output, &start_time, filter)
}

/// Parse macOS `FsEvents`
pub(crate) fn fseventsd(
    output: &mut Output,
    filter: bool,
    options: &FseventsOptions,
) -> Result<(), MacArtifactError> {
    let results = grab_fseventsd(options, filter, output);
    if results.is_err() {
        warn!(
            "[core] Failed to parse fseventsd: {:?}",
            results.unwrap_err()
        );
        return Err(MacArtifactError::FsEventsd);
    }
    Ok(())
}

/// Parse macOS `Launchd`
pub(crate) fn launchd(
    output: &mut Output,
    filter: bool,
    options: &LaunchdOptions,
) -> Result<(), MacArtifactError> {
    let start_time = time::time_now();

    let artifact_result = grab_launchd(options);
    let results = match artifact_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to parse launchd: {err:?}");
            return Err(MacArtifactError::Launchd);
        }
    };

    let serde_data_result = serde_json::to_value(results);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize launchd: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let output_name = "launchd";
    output_data(&mut serde_data, output_name, output, &start_time, filter)
}

/// Get macOS `Unifiedlogs`
pub(crate) fn unifiedlogs(
    output: &mut Output,
    filter: bool,
    options: &UnifiedLogsOptions,
) -> Result<(), MacArtifactError> {
    grab_logs(options, output, filter)
}

/// Get macOS `ExecPolicy`
pub(crate) fn execpolicy(
    output: &mut Output,
    filter: bool,
    options: &ExecPolicyOptions,
) -> Result<(), MacArtifactError> {
    let start_time = time::time_now();

    let artifact_result = grab_execpolicy(options);
    let results = match artifact_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to query execpolicy: {err:?}");
            return Err(MacArtifactError::ExecPolicy);
        }
    };

    let serde_data_result = serde_json::to_value(results);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize execpolicy: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let output_name = "execpolicy";
    output_data(&mut serde_data, output_name, output, &start_time, filter)
}

/// Parse sudo logs on macOS
pub(crate) fn sudo_logs_macos(
    output: &mut Output,
    filter: bool,
    options: &MacosSudoOptions,
) -> Result<(), MacArtifactError> {
    let start_time = time_now();
    let artifact_result = grab_sudo_logs(options);
    let results = match artifact_result {
        Ok(results) => results,
        Err(err) => {
            warn!("[core] Failed to get sudo log data: {err:?}");
            return Err(MacArtifactError::SudoLog);
        }
    };

    let serde_data_result = serde_json::to_value(results);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize sudo log data: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let output_name = "sudologs-macos";
    output_data(&mut serde_data, output_name, output, &start_time, filter)
}

/// Parse spotlight on macOS
pub(crate) fn spotlight(
    output: &mut Output,
    filter: bool,
    options: &SpotlightOptions,
) -> Result<(), MacArtifactError> {
    let artifact_result = grab_spotlight(options, output, filter);
    match artifact_result {
        Ok(results) => Ok(results),
        Err(err) => {
            warn!("[core] Failed to get spotlight data: {err:?}");
            Err(MacArtifactError::Spotlight)
        }
    }
}

/// Output macOS artifacts
pub(crate) fn output_data(
    serde_data: &mut Value,
    output_name: &str,
    output: &mut Output,
    start_time: &u64,
    filter: bool,
) -> Result<(), MacArtifactError> {
    let status = output_artifact(serde_data, output_name, output, start_time, filter);
    if status.is_err() {
        error!("[core] Could not output data: {:?}", status.unwrap_err());
        return Err(MacArtifactError::Output);
    }
    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use crate::{
        artifacts::os::macos::artifacts::{
            emond, execpolicy, fseventsd, groups_macos, launchd, loginitems, output_data,
            spotlight, sudo_logs_macos, unifiedlogs, users_macos,
        },
        structs::{
            artifacts::os::macos::{
                EmondOptions, ExecPolicyOptions, FseventsOptions, LaunchdOptions,
                LoginitemsOptions, MacosGroupsOptions, MacosSudoOptions, MacosUsersOptions,
                SpotlightOptions, UnifiedLogsOptions,
            },
            toml::Output,
        },
        utils::time,
    };
    use serde_json::json;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            timeline: false,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_loginitems() {
        let mut output = output_options("loginitems_test", "local", "./tmp", false);

        let status = loginitems(&mut output, false, &LoginitemsOptions { alt_file: None }).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_emond() {
        let mut output = output_options("emond_test", "local", "./tmp", false);

        let status = emond(&mut output, false, &EmondOptions { alt_path: None }).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_users_macos() {
        let mut output = output_options("users_test", "local", "./tmp", false);

        let status =
            users_macos(&mut output, false, &&MacosUsersOptions { alt_path: None }).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_groups_macos() {
        let mut output = output_options("groups_test", "local", "./tmp", false);

        let status =
            groups_macos(&mut output, false, &&MacosGroupsOptions { alt_path: None }).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    #[ignore = "Takes a long time to run"]
    fn test_fseventsd() {
        let mut output = output_options("fseventsd_test", "local", "./tmp", false);

        let status = fseventsd(&mut output, false, &FseventsOptions { alt_file: None }).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_launchd() {
        let mut output = output_options("launchd_test", "local", "./tmp", false);

        let status = launchd(&mut output, false, &LaunchdOptions { alt_file: None }).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_unifiedlogs() {
        let mut output = output_options("unifiedlogs_test", "local", "./tmp", false);
        let sources = vec![String::from("Special")];

        let status = unifiedlogs(
            &mut output,
            false,
            &UnifiedLogsOptions {
                sources,
                logarchive_path: None,
            },
        )
        .unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_execpolicy() {
        let mut output = output_options("execpolicy_test", "local", "./tmp", true);

        let _status = execpolicy(&mut output, false, &ExecPolicyOptions { alt_file: None });
    }

    #[test]
    fn test_sudo_logs_macos() {
        let mut output = output_options("sudologs", "local", "./tmp", false);

        let status = sudo_logs_macos(
            &mut output,
            false,
            &&MacosSudoOptions {
                logarchive_path: None,
            },
        )
        .unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_spotlight() {
        let mut output = output_options("spotlight", "local", "./tmp", false);

        let status = spotlight(
            &mut output,
            false,
            &SpotlightOptions {
                alt_path: None,
                include_additional: None,
            },
        )
        .unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data() {
        let mut output = output_options("output_test", "local", "./tmp", false);
        let start_time = time::time_now();

        let name = "test";
        let mut data = json!({"test":"test"});
        let status = output_data(&mut data, name, &mut output, &start_time, false).unwrap();
        assert_eq!(status, ());
    }
}
