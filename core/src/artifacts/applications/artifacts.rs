use super::error::ApplicationError;
use crate::{artifacts::output::output_artifact, structs::toml::Output, utils::time};
use log::{error, warn};
use serde_json::Value;

/// Parse macOS Safari history
pub(crate) fn safari_history(output: &mut Output, filter: &bool) -> Result<(), ApplicationError> {
    use super::safari::history::get_safari_history;

    let start_time = time::time_now();

    let history_results = get_safari_history();
    let history_data = match history_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[core] Artemis macOS failed to get Safari history: {err:?}");
            return Err(ApplicationError::SafariHistory);
        }
    };

    let serde_data_result = serde_json::to_value(history_data);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize Safari history: {err:?}");
            return Err(ApplicationError::Serialize);
        }
    };

    let output_name = "safari_history";
    output_data(&mut serde_data, output_name, output, &start_time, filter)
}

/// Parse macOS Safari downloads
pub(crate) fn safari_downloads(output: &mut Output, filter: &bool) -> Result<(), ApplicationError> {
    use super::safari::downloads::get_safari_downloads;

    let start_time = time::time_now();

    let download_results = get_safari_downloads();
    let download_data = match download_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[core] Artemis macOS failed to get Safari downloads: {err:?}");
            return Err(ApplicationError::SafariDownloads);
        }
    };

    let serde_data_result = serde_json::to_value(download_data);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize Safari downloads: {err:?}");
            return Err(ApplicationError::Serialize);
        }
    };

    let output_name = "safari_downloads";
    output_data(&mut serde_data, output_name, output, &start_time, filter)
}

// Output application artifacts
pub(crate) fn output_data(
    serde_data: &mut Value,
    output_name: &str,
    output: &mut Output,
    start_time: &u64,
    filter: &bool,
) -> Result<(), ApplicationError> {
    let status = output_artifact(serde_data, output_name, output, start_time, filter);
    if status.is_err() {
        error!("[core] Could not output data: {:?}", status.unwrap_err());
        return Err(ApplicationError::Output);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::artifacts::applications::artifacts::{safari_downloads, safari_history};
    use crate::structs::toml::Output;

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
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
    fn test_safari_history() {
        let mut output = output_options("safari_test", "local", "./tmp", false);

        let status = safari_history(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_safari_downloads() {
        let mut output = output_options("safari_test", "local", "./tmp", false);

        let status = safari_downloads(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }
}
