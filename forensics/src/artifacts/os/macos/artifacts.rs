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
    output::{manager::OutputManager, record::serialize_records_to_stream},
    structs::artifacts::os::macos::{
        EmondOptions, ExecPolicyOptions, FseventsOptions, LaunchdOptions, LoginitemsOptions,
        MacosGroupsOptions, MacosSudoOptions, MacosUsersOptions, SpotlightOptions,
        UnifiedLogsOptions,
    },
};
use tracing::{error, warn};

/// Parse macOS `LoginItems`
pub(crate) fn loginitems(
    manager: &mut OutputManager,
    options: &LoginitemsOptions,
) -> Result<(), MacArtifactError> {
    let artifact_result = grab_loginitems(options);
    let entries = match artifact_result {
        Ok(results) => results,
        Err(err) => {
            error!("Failed to parse loginitems: {err:?}");
            return Err(MacArtifactError::LoginItem);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }
    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to serialize loginitems: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let artifact_name = "loginitems";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("Failed to output loginitems: {err:?}");
        return Err(MacArtifactError::Output);
    }

    Ok(())
}

/// Parse macOS `Emond`
pub(crate) fn emond(
    manager: &mut OutputManager,
    options: &EmondOptions,
) -> Result<(), MacArtifactError> {
    let results = grab_emond(options);
    let entries = match results {
        Ok(result) => result,
        Err(err) => {
            warn!("Failed to parse emond rules: {err:?}");
            return Err(MacArtifactError::Emond);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(results) => results,
        Err(err) => {
            error!("Failed to serialize emond: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let artifact_name = "emond";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("Failed to output emond: {err:?}");
        return Err(MacArtifactError::Output);
    }

    Ok(())
}

/// Get macOS `Users`
pub(crate) fn users_macos(
    manager: &mut OutputManager,
    options: &MacosUsersOptions,
) -> Result<(), MacArtifactError> {
    let entries = grab_users(options);
    if entries.is_empty() {
        return Ok(());
    }
    let mut records = match serialize_records_to_stream(entries) {
        Ok(results) => results,
        Err(err) => {
            error!("Failed to serialize users: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let artifact_name = "users-macos";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("Failed to output users: {err:?}");
        return Err(MacArtifactError::Output);
    }

    Ok(())
}

/// Get macOS `Groups`
pub(crate) fn groups_macos(
    manager: &mut OutputManager,
    options: &MacosGroupsOptions,
) -> Result<(), MacArtifactError> {
    let entries = grab_groups(options);
    if entries.is_empty() {
        return Ok(());
    }
    let mut records = match serialize_records_to_stream(entries) {
        Ok(results) => results,
        Err(err) => {
            error!("Failed to serialize groups: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let artifact_name = "groups-macos";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("Failed to output groups: {err:?}");
        return Err(MacArtifactError::Output);
    }

    Ok(())
}

/// Parse macOS `FsEvents`
pub(crate) fn fseventsd(
    manager: &mut OutputManager,
    options: &FseventsOptions,
) -> Result<(), MacArtifactError> {
    if let Err(err) = grab_fseventsd(options, manager) {
        warn!("Failed to parse fseventsd: {err:?}");
        return Err(MacArtifactError::FsEventsd);
    }

    Ok(())
}

/// Parse macOS `Launchd`
pub(crate) fn launchd(
    manager: &mut OutputManager,
    options: &LaunchdOptions,
) -> Result<(), MacArtifactError> {
    let artifact_result = grab_launchd(options);
    let entries = match artifact_result {
        Ok(results) => results,
        Err(err) => {
            error!("Failed to parse launchd: {err:?}");
            return Err(MacArtifactError::Launchd);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(results) => results,
        Err(err) => {
            error!("Failed to serialize launchd: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let artifact_name = "launchd";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("Failed to output launchd: {err:?}");
        return Err(MacArtifactError::Output);
    }

    Ok(())
}

/// Get macOS `Unifiedlogs`
pub(crate) fn unifiedlogs(
    manager: &mut OutputManager,
    options: &UnifiedLogsOptions,
) -> Result<(), MacArtifactError> {
    grab_logs(options, manager)
}

/// Get macOS `ExecPolicy`
pub(crate) fn execpolicy(
    manager: &mut OutputManager,
    options: &ExecPolicyOptions,
) -> Result<(), MacArtifactError> {
    let artifact_result = grab_execpolicy(options);
    let entries = match artifact_result {
        Ok(results) => results,
        Err(err) => {
            error!("Failed to query execpolicy: {err:?}");
            return Err(MacArtifactError::ExecPolicy);
        }
    };
    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(results) => results,
        Err(err) => {
            error!("Failed to serialize execpolicy: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let artifact_name = "execpolicy";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("Failed to output execpolicy: {err:?}");
        return Err(MacArtifactError::Output);
    }

    Ok(())
}

/// Parse sudo logs on macOS
pub(crate) fn sudo_logs_macos(
    manager: &mut OutputManager,
    options: &MacosSudoOptions,
) -> Result<(), MacArtifactError> {
    let artifact_result = grab_sudo_logs(options);
    let entries = match artifact_result {
        Ok(results) => results,
        Err(err) => {
            warn!("Failed to get sudo log data: {err:?}");
            return Err(MacArtifactError::SudoLog);
        }
    };

    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(results) => results,
        Err(err) => {
            error!("Failed to serialize sudo log data: {err:?}");
            return Err(MacArtifactError::Serialize);
        }
    };

    let artifact_name = "sudologs-macos";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("Failed to output sudo logs: {err:?}");
        return Err(MacArtifactError::Output);
    }

    Ok(())
}

/// Parse spotlight on macOS
pub(crate) fn spotlight(
    manager: &mut OutputManager,
    options: &SpotlightOptions,
) -> Result<(), MacArtifactError> {
    if let Err(err) = grab_spotlight(options, manager) {
        warn!("Failed to get spotlight data: {err:?}");
        return Err(MacArtifactError::Spotlight);
    }

    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        artifacts::os::macos::artifacts::{
            emond, execpolicy, fseventsd, groups_macos, launchd, loginitems, spotlight,
            sudo_logs_macos, unifiedlogs, users_macos,
        },
        output::manager::OutputManager,
        structs::artifacts::os::macos::{
            EmondOptions, ExecPolicyOptions, FseventsOptions, LaunchdOptions, LoginitemsOptions,
            MacosGroupsOptions, MacosSudoOptions, MacosUsersOptions, SpotlightOptions,
            UnifiedLogsOptions,
        },
    };
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_loginitems() {
        let mut output = output_options("loginitems_test", "./tmp", false);

        let status = loginitems(&mut output, &LoginitemsOptions { alt_file: None }).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_emond() {
        let mut output = output_options("emond_test", "./tmp", false);

        let status = emond(&mut output, &EmondOptions { alt_dir: None }).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_users_macos() {
        let mut output = output_options("users_test", "./tmp", false);

        let status = users_macos(&mut output, &MacosUsersOptions { alt_dir: None }).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_groups_macos() {
        let mut output = output_options("groups_test", "./tmp", false);

        let status = groups_macos(&mut output, &MacosGroupsOptions { alt_dir: None }).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    #[ignore = "Takes a long time to run"]
    fn test_fseventsd() {
        let mut output = output_options("fseventsd_test", "./tmp", false);

        let status = fseventsd(&mut output, &FseventsOptions { alt_file: None }).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_launchd() {
        let mut output = output_options("launchd_test", "./tmp", false);

        let status = launchd(&mut output, &LaunchdOptions { alt_file: None }).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_unifiedlogs() {
        let mut output = output_options("unifiedlogs_test", "./tmp", false);
        let sources = vec![String::from("Special")];

        let status = unifiedlogs(
            &mut output,
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
        let mut output = output_options("execpolicy_test", "./tmp", true);

        let _status = execpolicy(&mut output, &ExecPolicyOptions { alt_file: None });
    }

    #[test]
    fn test_sudo_logs_macos() {
        let mut output = output_options("sudologs", "./tmp", false);

        let status = sudo_logs_macos(
            &mut output,
            &MacosSudoOptions {
                logarchive_path: None,
            },
        )
        .unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_spotlight() {
        let mut output = output_options("spotlight", "./tmp", false);

        let status = spotlight(
            &mut output,
            &SpotlightOptions {
                alt_dir: None,
                include_additional: None,
            },
        )
        .unwrap();
        assert_eq!(status, ());
    }
}
