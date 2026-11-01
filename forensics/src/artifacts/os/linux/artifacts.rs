use crate::artifacts::os::linux::error::LinuxArtifactError;
use crate::output::manager::OutputManager;
use crate::output::record::serialize_records_to_stream;
use crate::structs::artifacts::os::linux::{JournalOptions, LinuxSudoOptions, LogonOptions};
use tracing::{error, warn};

use super::sudo::logs::grab_sudo_logs;
use super::{journals::parser::grab_journal, logons::parser::grab_logons};

/// Get Linux `Journals`
pub(crate) fn journals(
    manager: &mut OutputManager,
    options: &JournalOptions,
) -> Result<(), LinuxArtifactError> {
    if let Err(err) = grab_journal(manager, options) {
        error!("Failed to get journals: {err:?}");
        return Err(LinuxArtifactError::Journal);
    }

    Ok(())
}

/// Get Linux `Logon` info
pub(crate) fn logons(
    manager: &mut OutputManager,
    options: &LogonOptions,
) -> Result<(), LinuxArtifactError> {
    let entries = grab_logons(options);
    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to serialize logons: {err:?}");
            return Err(LinuxArtifactError::Serialize);
        }
    };

    let artifact_name = "logons";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("Failed to output logons: {err:?}");
        return Err(LinuxArtifactError::Output);
    }

    Ok(())
}

/// Parse sudo logs on Linux
pub(crate) fn sudo_logs_linux(
    manager: &mut OutputManager,
    options: &LinuxSudoOptions,
) -> Result<(), LinuxArtifactError> {
    let sudo_results = grab_sudo_logs(options);
    let entries = match sudo_results {
        Ok(results) => results,
        Err(err) => {
            warn!("Failed to get sudo log data: {err:?}");
            return Err(LinuxArtifactError::SudoLog);
        }
    };
    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to serialize sudo log data: {err:?}");
            return Err(LinuxArtifactError::Serialize);
        }
    };

    let artifact_name = "sudologs-linux";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("Failed to output sudologs-linux: {err:?}");
        return Err(LinuxArtifactError::Output);
    }

    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use crate::artifacts::os::linux::artifacts::{journals, logons, sudo_logs_linux};
    use crate::artifacts::os::systeminfo::info::get_info_metadata;
    use crate::output::manager::OutputManager;
    use crate::structs::artifacts::os::linux::{JournalOptions, LinuxSudoOptions, LogonOptions};
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
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
    fn test_journals() {
        let mut output = output_options("journals_test", "./tmp", false);

        let status = journals(
            &mut output,
            &JournalOptions {
                alt_dir: Some(String::from("./tmp")),
            },
        )
        .unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_logons() {
        let mut output = output_options("logons_test", "./tmp", false);

        let status = logons(
            &mut output,
            &LogonOptions {
                alt_file: Some(String::from("/var/run/utmp")),
            },
        )
        .unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_sudo_logs_linux() {
        let mut output = output_options("sudologs", "./tmp", false);

        let status = sudo_logs_linux(
            &mut output,
            &LinuxSudoOptions {
                alt_dir: Some(String::from("./tmp")),
            },
        )
        .unwrap();
        assert_eq!(status, ());
    }
}
