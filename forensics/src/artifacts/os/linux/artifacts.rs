use crate::artifacts::os::linux::error::LinuxArtifactError;
use crate::artifacts::os::linux::ext4::parser::ext4_filelisting;
use crate::artifacts::output::output_artifact;
use crate::structs::artifacts::os::linux::{
    Ext4Options, JournalOptions, LinuxSudoOptions, LogonOptions,
};
use crate::structs::toml::Output;
use crate::utils::time;
use log::{error, warn};
use serde_json::Value;

use super::sudo::logs::grab_sudo_logs;
use super::{journals::parser::grab_journal, logons::parser::grab_logons};

/// Get Linux `Journals`
pub(crate) fn journals(
    output: &mut Output,
    filter: bool,
    options: &JournalOptions,
) -> Result<(), LinuxArtifactError> {
    let start_time = time::time_now();

    let artifact_result = grab_journal(output, start_time, filter, options);
    match artifact_result {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[forensics] Failed to get journals: {err:?}");
            Err(LinuxArtifactError::Journal)
        }
    }
}

/// Get Linux `Logon` info
pub(crate) fn logons(
    output: &mut Output,
    filter: bool,
    options: &LogonOptions,
) -> Result<(), LinuxArtifactError> {
    let start_time = time::time_now();

    let result = grab_logons(options);
    let serde_data_result = serde_json::to_value(result);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize logons: {err:?}");
            return Err(LinuxArtifactError::Serialize);
        }
    };

    let output_name = "logons";
    output_data(&mut serde_data, output_name, output, start_time, filter)
}

/// Parse sudo logs on Linux
pub(crate) fn sudo_logs_linux(
    output: &mut Output,
    filter: bool,
    options: &LinuxSudoOptions,
) -> Result<(), LinuxArtifactError> {
    let start_time = time::time_now();

    let sudo_results = grab_sudo_logs(options);
    let sudo_data = match sudo_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[forensics] Failed to get sudo log data: {err:?}");
            return Err(LinuxArtifactError::SudoLog);
        }
    };

    let serde_data_result = serde_json::to_value(sudo_data);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[forensics] Failed to serialize sudo log data: {err:?}");
            return Err(LinuxArtifactError::Serialize);
        }
    };

    let output_name = "sudologs-linux";
    output_data(&mut serde_data, output_name, output, start_time, filter)
}

/// Parse the ext4 filesystem
pub(crate) fn ext4_filelist(
    output: &mut Output,
    filter: bool,
    options: &Ext4Options,
) -> Result<(), LinuxArtifactError> {
    let artifact_result = ext4_filelisting(options, output, filter);
    match artifact_result {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[forensics] Failed to get ext4 filelisting: {err:?}");
            Err(LinuxArtifactError::Ext4)
        }
    }
}

/// Output Linux artifacts
pub(crate) fn output_data(
    serde_data: &mut Value,
    output_name: &str,
    output: &mut Output,
    start_time: u64,
    filter: bool,
) -> Result<(), LinuxArtifactError> {
    let status = output_artifact(serde_data, output_name, output, start_time, filter);
    if let Err(result) = status {
        error!("[forensics] Could not output data: {result:?}");
        return Err(LinuxArtifactError::Output);
    }
    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use crate::artifacts::os::linux::artifacts::{
        ext4_filelist, journals, logons, output_data, sudo_logs_linux,
    };
    use crate::artifacts::os::systeminfo::info::get_info_metadata;
    use crate::structs::artifacts::os::linux::{
        Ext4Options, JournalOptions, LinuxSudoOptions, LogonOptions,
    };
    use crate::structs::toml::Output;
    use crate::utils::time;
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
    fn test_output_data() {
        let mut output = output_options("output_test", "local", "./tmp", false);
        let start_time = time::time_now();

        let name = "test";
        let mut data = json!({"test":"test"});
        let status = output_data(&mut data, name, &mut output, start_time, false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_journals() {
        let mut output = output_options("journals_test", "local", "./tmp", false);

        let status = journals(
            &mut output,
            false,
            &JournalOptions {
                alt_path: Some(String::from("./tmp")),
            },
        )
        .unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_logons() {
        let mut output = output_options("logons_test", "local", "./tmp", false);

        let status = logons(
            &mut output,
            false,
            &LogonOptions {
                alt_file: Some(String::from("/var/run/utmp")),
            },
        )
        .unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_sudo_logs_linux() {
        let mut output = output_options("sudologs", "local", "./tmp", false);

        let status = sudo_logs_linux(
            &mut output,
            false,
            &LinuxSudoOptions {
                alt_path: Some(String::from("./tmp")),
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
        let mut output = output_options("ext4", "local", "./tmp", false);
        ext4_filelist(
            &mut output,
            false,
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
