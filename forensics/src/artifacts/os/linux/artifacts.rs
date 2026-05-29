use crate::artifacts::os::linux::error::LinuxArtifactError;
use crate::artifacts::os::linux::ext4::parser::ext4_filelisting;
use crate::output2::manager::OutputManager;
use crate::output2::record::serialize_records_to_stream;
use crate::structs::artifacts::os::linux::{
    Ext4Options, JournalOptions, LinuxSudoOptions, LogonOptions,
};
use log::{error, warn};

use super::sudo::logs::grab_sudo_logs;
use super::{journals::parser::grab_journal, logons::parser::grab_logons};

/// Get Linux `Journals`
pub(crate) fn journals(
    manager: &mut OutputManager,
    options: &JournalOptions,
) -> Result<(), LinuxArtifactError> {
    if let Err(err) = grab_journal(manager, options) {
        error!("[forensics] Failed to get journals: {err:?}");
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
            error!("[forensics] Failed to serialize logons: {err:?}");
            return Err(LinuxArtifactError::Serialize);
        }
    };

    let artifact_name = "logons";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output logons: {err:?}");
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
            warn!("[forensics] Failed to get sudo log data: {err:?}");
            return Err(LinuxArtifactError::SudoLog);
        }
    };
    if entries.is_empty() {
        return Ok(());
    }

    let mut records = match serialize_records_to_stream(entries) {
        Ok(result) => result,
        Err(err) => {
            error!("[forensics] Failed to serialize sudo log data: {err:?}");
            return Err(LinuxArtifactError::Serialize);
        }
    };

    let artifact_name = "sudologs-linux";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[forensics] Failed to output sudologs-linux: {err:?}");
    }

    Ok(())
}

/// Parse the ext4 filesystem
pub(crate) fn ext4_filelist(
    manager: &mut OutputManager,
    options: &Ext4Options,
) -> Result<(), LinuxArtifactError> {
    if let Err(err) = ext4_filelisting(options, manager) {
        error!("[forensics] Failed to get ext4 filelisting: {err:?}");
        return Err(LinuxArtifactError::Ext4);
    }

    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use crate::artifacts::os::linux::artifacts::{
        ext4_filelist, journals, logons, sudo_logs_linux,
    };
    use crate::artifacts::os::systeminfo::info::get_info_metadata;
    use crate::output2::config::{OutputConfig, OutputDestination, OutputFormat};
    use crate::output2::manager::OutputManager;
    use crate::structs::artifacts::os::linux::{
        Ext4Options, JournalOptions, LinuxSudoOptions, LogonOptions,
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

    #[test]
    fn test_ext4_filelist() {
        // Run test only in Github CI. Parsing the ext4 filesystem requires root
        if !get_info_metadata().kernel_version.contains("azure") {
            return;
        }
        let mut output = output_options("ext4", "./tmp", false);
        ext4_filelist(
            &mut output,
            &Ext4Options {
                start_path: String::from("/"),
                depth: 99,
                device: None,
                md5: None,
                sha1: None,
                sha256: None,
                path_regex: None,
                filename_regex: None,
            },
        )
        .unwrap();
    }
}
