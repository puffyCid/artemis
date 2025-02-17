use super::{
    chromium::{downloads::get_chromium_downloads, history::get_chromium_history},
    error::ApplicationError,
    firefox::{downloads::get_firefox_downloads, history::get_firefox_history},
};
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

/// Parse `Firefox` history
pub(crate) fn firefox_history(output: &mut Output, filter: &bool) -> Result<(), ApplicationError> {
    let start_time = time::time_now();
    let history_results = get_firefox_history();

    let history_data = match history_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[core] Artemis macOS failed to get Firefox history: {err:?}");
            return Err(ApplicationError::FirefoxHistory);
        }
    };

    let serde_data_result = serde_json::to_value(history_data);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize Firefox history: {err:?}");
            return Err(ApplicationError::Serialize);
        }
    };

    let output_name = "firefox_history";
    output_data(&mut serde_data, output_name, output, &start_time, filter)
}

/// Parse `Firefox` downloads
pub(crate) fn firefox_downloads(
    output: &mut Output,
    filter: &bool,
) -> Result<(), ApplicationError> {
    let start_time = time::time_now();
    let download_results = get_firefox_downloads();

    let download_data = match download_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[core] Artemis failed to get Firefox downloads: {err:?}");
            return Err(ApplicationError::FirefoxDownloads);
        }
    };

    let serde_data_result = serde_json::to_value(download_data);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize Firefox downloads: {err:?}");
            return Err(ApplicationError::Serialize);
        }
    };

    let output_name = "firefox_downloads";
    output_data(&mut serde_data, output_name, output, &start_time, filter)
}

/// Parse Chromium history
pub(crate) fn chromium_history(output: &mut Output, filter: &bool) -> Result<(), ApplicationError> {
    let start_time = time::time_now();

    let history_results = get_chromium_history();
    let history_data = match history_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[core] Artemis failed to get Chromium history: {err:?}");
            return Err(ApplicationError::ChromiumHistory);
        }
    };

    let serde_data_result = serde_json::to_value(history_data);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize Chromium history: {err:?}");
            return Err(ApplicationError::Serialize);
        }
    };

    let output_name = "chromium_history";
    output_data(&mut serde_data, output_name, output, &start_time, filter)
}

/// Parse Chromium downloads
pub(crate) fn chromium_downloads(
    output: &mut Output,
    filter: &bool,
) -> Result<(), ApplicationError> {
    let start_time = time::time_now();

    let download_results = get_chromium_downloads();
    let download_data = match download_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[core] Artemis failed to get Chromium downloads: {err:?}");
            return Err(ApplicationError::ChromiumDownloads);
        }
    };

    let serde_data_result = serde_json::to_value(download_data);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[core] Failed to serialize Chromium downloads: {err:?}");
            return Err(ApplicationError::Serialize);
        }
    };

    let output_name = "chromium_downloads";
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
    use crate::{
        artifacts::applications::artifacts::{
            chromium_downloads, chromium_history, firefox_downloads, firefox_history,
        },
        structs::toml::Output,
    };

    #[cfg(target_os = "macos")]
    use crate::artifacts::applications::artifacts::{safari_downloads, safari_history};

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
    #[cfg(target_os = "macos")]
    fn test_safari_history() {
        let mut output = output_options("safari_test", "local", "./tmp", false);

        let status = safari_history(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_safari_downloads() {
        let mut output = output_options("safari_test", "local", "./tmp", false);

        let status = safari_downloads(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_firefox_history() {
        let mut output = output_options("firefox_test", "local", "./tmp", false);

        let _ = firefox_history(&mut output, &false).unwrap();
    }

    #[test]
    fn test_firefox_downloads() {
        let mut output = output_options("firefox_test", "local", "./tmp", false);

        let _ = firefox_downloads(&mut output, &false).unwrap();
    }

    #[test]
    fn test_chromium_history() {
        let mut output = output_options("chromium_test", "local", "./tmp", false);

        let status = chromium_history(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_chromium_downloads() {
        let mut output = output_options("chromium_test", "local", "./tmp", false);

        let status = chromium_downloads(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }
}
