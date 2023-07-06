use super::{
    cron::crontab,
    error::UnixArtifactError,
    shell_history::{bash::BashHistory, python::PythonHistory, zsh::ZshHistory},
    sudo::linux::grab_sudo_logs,
};
use crate::{
    output::formats::{json::json_format, jsonl::jsonl_format},
    runtime::deno::filter_script,
    utils::{artemis_toml::Output, time},
};
use log::{error, warn};
use serde_json::Value;

/// Get zsh history depending on target OS
pub(crate) fn zsh_history(output: &mut Output, filter: &bool) -> Result<(), UnixArtifactError> {
    let start_time = time::time_now();
    let zsh_results = ZshHistory::get_user_zsh_history();
    let history_data = match zsh_results {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Artemis failed to get zsh history: {err:?}");
            return Err(UnixArtifactError::Zsh);
        }
    };

    let serde_data_result = serde_json::to_value(history_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize zsh history: {err:?}");
            return Err(UnixArtifactError::Serialize);
        }
    };

    let output_name = "zsh_history";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get bash history depending on target OS
pub(crate) fn bash_history(output: &mut Output, filter: &bool) -> Result<(), UnixArtifactError> {
    let start_time = time::time_now();

    let bash_results = BashHistory::get_user_bash_history();
    let history_data = match bash_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[artemis-core] Artemis unix failed to get bash history: {err:?}");
            return Err(UnixArtifactError::Bash);
        }
    };

    let serde_data_result = serde_json::to_value(history_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize bash history: {err:?}");
            return Err(UnixArtifactError::Serialize);
        }
    };

    let output_name = "bash_history";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Get python history depending on target OS
pub(crate) fn python_history(output: &mut Output, filter: &bool) -> Result<(), UnixArtifactError> {
    let start_time = time::time_now();

    let bash_results = PythonHistory::get_user_python_history();
    let history_data = match bash_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[artemis-core] Artemis unix failed to get python history: {err:?}");
            return Err(UnixArtifactError::Python);
        }
    };

    let serde_data_result = serde_json::to_value(history_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize python history: {err:?}");
            return Err(UnixArtifactError::Serialize);
        }
    };

    let output_name = "python_history";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Parse cron data
pub(crate) fn cron_job(output: &mut Output, filter: &bool) -> Result<(), UnixArtifactError> {
    let start_time = time::time_now();

    let cron_results = crontab::Cron::parse_cron();
    let cron_data = match cron_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[artemis-core] Artemis unix failed to get cron data: {err:?}");
            return Err(UnixArtifactError::Cron);
        }
    };

    let serde_data_result = serde_json::to_value(cron_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize cron data: {err:?}");
            return Err(UnixArtifactError::Serialize);
        }
    };

    let output_name = "cron";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Parse sudo logs on a Unix system
pub(crate) fn sudo_logs(output: &mut Output, filter: &bool) -> Result<(), UnixArtifactError> {
    let start_time = time::time_now();

    let cron_results = grab_sudo_logs();
    let cron_data = match cron_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[artemis-core] Artemis unix failed to get sudo log data: {err:?}");
            return Err(UnixArtifactError::SudoLog);
        }
    };

    let serde_data_result = serde_json::to_value(cron_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize sudo log data: {err:?}");
            return Err(UnixArtifactError::Serialize);
        }
    };

    let output_name = "sudologs";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

// Output unix artifacts
pub(crate) fn output_data(
    serde_data: &Value,
    output_name: &str,
    output: &mut Output,
    start_time: &u64,
    filter: &bool,
) -> Result<(), UnixArtifactError> {
    if *filter {
        if let Some(script) = &output.filter_script.clone() {
            let args = vec![serde_data.to_string(), output_name.to_string()];
            if let Some(name) = &output.filter_name.clone() {
                let filter_result = filter_script(output, &args, name, script);
                return match filter_result {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        error!(
                            "[artemis-core] Could not apply filter script to unix data: {err:?}"
                        );
                        Err(UnixArtifactError::FilterOutput)
                    }
                };
            }
            let filter_result = filter_script(output, &args, "UnknownFilterName", script);
            return match filter_result {
                Ok(_) => Ok(()),
                Err(err) => {
                    error!(
                    "[artemis-core] Could not apply unknown filter script to unix data: {err:?}"
                );
                    Err(UnixArtifactError::FilterOutput)
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
        return Err(UnixArtifactError::Format);
    };
    match output_status {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Could not output data: {err:?}");
            return Err(UnixArtifactError::Output);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::output_data;
    use crate::{
        artifacts::os::unix::artifacts::{
            bash_history, cron_job, python_history, sudo_logs, zsh_history,
        },
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
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_zsh_history() {
        let mut output = output_options("zsh_history", "local", "./tmp", false);

        let status = zsh_history(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_bash_history() {
        let mut output = output_options("bash_history", "local", "./tmp", false);

        let _ = bash_history(&mut output, &false).unwrap();
    }

    #[test]
    fn test_python_history() {
        let mut output = output_options("python_history", "local", "./tmp", false);

        let status = python_history(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_cron_job() {
        let mut output = output_options("cron", "local", "./tmp", false);

        let status = cron_job(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_sudo_logs() {
        let mut output = output_options("sudologs", "local", "./tmp", false);

        let status = sudo_logs(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_output_data() {
        let mut output = output_options("output_test", "local", "./tmp", false);
        let start_time = time::time_now();

        let name = "test";
        let data = serde_json::Value::String(String::from("test"));
        let status = output_data(&data, name, &mut output, &start_time, &false).unwrap();
        assert_eq!(status, ());
    }
}
