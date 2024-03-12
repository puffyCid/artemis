use crate::artifacts::os::linux::error::LinuxArtifactError;
use crate::output::formats::json::json_format;
use crate::output::formats::jsonl::jsonl_format;
use crate::runtime::deno::filter_script;
use crate::structs::artifacts::os::linux::{JournalOptions, LinuxSudoOptions, LogonOptions};
use crate::structs::toml::Output;
use crate::utils::time;
use log::{error, warn};
use serde_json::Value;

use super::sudo::logs::grab_sudo_logs;
use super::{journals::parser::grab_journal, logons::parser::grab_logons};

/// Get Linux `Journals`
pub(crate) fn journals(
    output: &mut Output,
    filter: &bool,
    options: &JournalOptions,
) -> Result<(), LinuxArtifactError> {
    let start_time = time::time_now();

    let artifact_result = grab_journal(output, &start_time, filter, options);
    match artifact_result {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[artemis-core] Failed to get journals: {err:?}");
            Err(LinuxArtifactError::Journal)
        }
    }
}

/// Get Linux `Logon` info
pub(crate) fn logons(
    output: &mut Output,
    filter: &bool,
    options: &LogonOptions,
) -> Result<(), LinuxArtifactError> {
    let start_time = time::time_now();

    let result = grab_logons(options);
    let serde_data_result = serde_json::to_value(result);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize logons: {err:?}");
            return Err(LinuxArtifactError::Serialize);
        }
    };

    let output_name = "logons";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Parse sudo logs on Linux
pub(crate) fn sudo_logs_linux(
    output: &mut Output,
    filter: &bool,
    options: &LinuxSudoOptions,
) -> Result<(), LinuxArtifactError> {
    let start_time = time::time_now();

    let cron_results = grab_sudo_logs(options);
    let cron_data = match cron_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[artemis-core] Failed to get sudo log data: {err:?}");
            return Err(LinuxArtifactError::SudoLog);
        }
    };

    let serde_data_result = serde_json::to_value(cron_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize sudo log data: {err:?}");
            return Err(LinuxArtifactError::Serialize);
        }
    };

    let output_name = "sudologs-linux";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Output Linux artifacts
pub(crate) fn output_data(
    serde_data: &Value,
    output_name: &str,
    output: &mut Output,
    start_time: &u64,
    filter: &bool,
) -> Result<(), LinuxArtifactError> {
    if *filter {
        if let Some(script) = &output.filter_script.clone() {
            let args = vec![serde_data.to_string(), output_name.to_string()];
            if let Some(name) = &output.filter_name.clone() {
                let filter_result = filter_script(output, &args, name, script);
                return match filter_result {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        error!(
                            "[artemis-core] Could not apply filter script to linux data: {err:?}"
                        );
                        Err(LinuxArtifactError::FilterOutput)
                    }
                };
            }
            let filter_result = filter_script(output, &args, "UnknownFilterName", script);
            return match filter_result {
                Ok(_) => Ok(()),
                Err(err) => {
                    error!(
                    "[artemis-core] Could not apply unknown filter script to linux data: {err:?}"
                );
                    Err(LinuxArtifactError::FilterOutput)
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
        return Err(LinuxArtifactError::Format);
    };
    match output_status {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Could not output data: {err:?}");
            return Err(LinuxArtifactError::Output);
        }
    }
    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use crate::artifacts::os::linux::artifacts::{journals, logons, output_data, sudo_logs_linux};
    use crate::structs::artifacts::os::linux::{JournalOptions, LinuxSudoOptions, LogonOptions};
    use crate::structs::toml::Output;
    use crate::utils::time;

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
            logging: Some(String::new()),
        }
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

    #[test]
    fn test_journals() {
        let mut output = output_options("journals_test", "local", "./tmp", false);

        let status = journals(
            &mut output,
            &false,
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
            &false,
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
            &false,
            &LinuxSudoOptions {
                alt_path: Some(String::from("./tmp")),
            },
        )
        .unwrap();
        assert_eq!(status, ());
    }
}
