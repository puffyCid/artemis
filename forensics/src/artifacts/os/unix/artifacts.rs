use super::{
    cron::crontab::parse_cron,
    error::UnixArtifactError,
    shell_history::{
        bash::get_user_bash_history, python::get_user_python_history, zsh::get_user_zsh_history,
    },
};
use crate::{artifacts::output::output_artifact, structs::toml::Output, utils::time};
use log::{error, warn};
use serde_json::Value;

/// Get zsh history depending on target OS
pub(crate) async fn zsh_history(
    output: &mut Output,
    filter: bool,
) -> Result<(), UnixArtifactError> {
    let start_time = time::time_now();
    let zsh_results = get_user_zsh_history();
    let history_data = match zsh_results {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Artemis failed to get zsh history: {err:?}");
            return Err(UnixArtifactError::Zsh);
        }
    };

    let serde_data_result = serde_json::to_value(history_data);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize zsh history: {err:?}");
            return Err(UnixArtifactError::Serialize);
        }
    };

    let output_name = "zsh_history";
    output_data(&mut serde_data, output_name, output, start_time, filter).await
}

/// Get bash history depending on target OS
pub(crate) async fn bash_history(
    output: &mut Output,
    filter: bool,
) -> Result<(), UnixArtifactError> {
    let start_time = time::time_now();

    let bash_results = get_user_bash_history();
    let history_data = match bash_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[core] Artemis unix failed to get bash history: {err:?}");
            return Err(UnixArtifactError::Bash);
        }
    };

    let serde_data_result = serde_json::to_value(history_data);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize bash history: {err:?}");
            return Err(UnixArtifactError::Serialize);
        }
    };

    let output_name = "bash_history";
    output_data(&mut serde_data, output_name, output, start_time, filter).await
}

/// Get python history depending on target OS
pub(crate) async fn python_history(
    output: &mut Output,
    filter: bool,
) -> Result<(), UnixArtifactError> {
    let start_time = time::time_now();

    let bash_results = get_user_python_history();
    let history_data = match bash_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[core] Artemis unix failed to get python history: {err:?}");
            return Err(UnixArtifactError::Python);
        }
    };

    let serde_data_result = serde_json::to_value(history_data);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize python history: {err:?}");
            return Err(UnixArtifactError::Serialize);
        }
    };

    let output_name = "python_history";
    output_data(&mut serde_data, output_name, output, start_time, filter).await
}

/// Parse cron data
pub(crate) async fn cron_job(output: &mut Output, filter: bool) -> Result<(), UnixArtifactError> {
    let start_time = time::time_now();

    let cron_results = parse_cron();
    let cron_data = match cron_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[core] Artemis unix failed to get cron data: {err:?}");
            return Err(UnixArtifactError::Cron);
        }
    };

    let serde_data_result = serde_json::to_value(cron_data);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize cron data: {err:?}");
            return Err(UnixArtifactError::Serialize);
        }
    };

    let output_name = "cron";
    output_data(&mut serde_data, output_name, output, start_time, filter).await
}

// Output unix artifacts
pub(crate) async fn output_data(
    serde_data: &mut Value,
    output_name: &str,
    output: &mut Output,
    start_time: u64,
    filter: bool,
) -> Result<(), UnixArtifactError> {
    let status = output_artifact(serde_data, output_name, output, start_time, filter).await;
    if status.is_err() {
        error!("[core] Could not output data: {:?}", status.unwrap_err());
        return Err(UnixArtifactError::Output);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::output_data;
    use crate::{
        artifacts::os::unix::artifacts::{bash_history, cron_job, python_history, zsh_history},
        structs::toml::Output,
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

    #[tokio::test]
    async fn test_zsh_history() {
        let mut output = output_options("zsh_history", "local", "./tmp", false);

        let status = zsh_history(&mut output, false).await.unwrap();
        assert_eq!(status, ());
    }

    #[tokio::test]
    async fn test_bash_history() {
        let mut output = output_options("bash_history", "local", "./tmp", false);

        let _ = bash_history(&mut output, false).await.unwrap();
    }

    #[tokio::test]
    async fn test_python_history() {
        let mut output = output_options("python_history", "local", "./tmp", false);

        let status = python_history(&mut output, false).await.unwrap();
        assert_eq!(status, ());
    }

    #[tokio::test]
    async fn test_cron_job() {
        let mut output = output_options("cron", "local", "./tmp", false);

        let status = cron_job(&mut output, false).await.unwrap();
        assert_eq!(status, ());
    }

    #[tokio::test]
    async fn test_output_data() {
        let mut output = output_options("output_test", "local", "./tmp", false);
        let start_time = time::time_now();

        let name = "test";
        let mut data = json!({"test":"test"});
        let status = output_data(&mut data, name, &mut output, start_time, false)
            .await
            .unwrap();
        assert_eq!(status, ());
    }
}
