use super::{
    chromium::{downloads::ChromiumDownloads, history::ChromiumHistory},
    error::ApplicationError,
    firefox::{downloads::FirefoxDownloads, history::FirefoxHistory},
};
use crate::{
    output::formats::{json::json_format, jsonl::jsonl_format},
    runtime::deno::filter_script,
    utils::{artemis_toml::Output, time},
};
use log::{error, warn};
use serde_json::Value;

/// Parse macOS Safari history
#[cfg(target_os = "macos")]
pub(crate) fn safari_history(output: &mut Output, filter: &bool) -> Result<(), ApplicationError> {
    use super::safari::history::SafariHistory;

    let start_time = time::time_now();

    let history_results = SafariHistory::get_history();
    let history_data = match history_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[artemis-core] Artemis macOS failed to get Safari history: {err:?}");
            return Err(ApplicationError::SafariHistory);
        }
    };

    let serde_data_result = serde_json::to_value(history_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize Safari history: {err:?}");
            return Err(ApplicationError::Serialize);
        }
    };

    let output_name = "safari_history";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Parse macOS Safari downloads
#[cfg(target_os = "macos")]
pub(crate) fn safari_downloads(output: &mut Output, filter: &bool) -> Result<(), ApplicationError> {
    use super::safari::downloads::SafariDownloads;

    let start_time = time::time_now();

    let download_results = SafariDownloads::get_downloads();
    let download_data = match download_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[artemis-core] Artemis macOS failed to get Safari downloads: {err:?}");
            return Err(ApplicationError::SafariDownloads);
        }
    };

    let serde_data_result = serde_json::to_value(download_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize Safari downloads: {err:?}");
            return Err(ApplicationError::Serialize);
        }
    };

    let output_name = "safari_downloads";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Parse `Firefox` history
pub(crate) fn firefox_history(output: &mut Output, filter: &bool) -> Result<(), ApplicationError> {
    let start_time = time::time_now();
    let history_results = FirefoxHistory::get_history();

    let history_data = match history_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[artemis-core] Artemis macOS failed to get Firefox history: {err:?}");
            return Err(ApplicationError::FirefoxHistory);
        }
    };

    let serde_data_result = serde_json::to_value(history_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize Firefox history: {err:?}");
            return Err(ApplicationError::Serialize);
        }
    };

    let output_name = "firefox_history";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Parse `Firefox` downloads
pub(crate) fn firefox_downloads(
    output: &mut Output,
    filter: &bool,
) -> Result<(), ApplicationError> {
    let start_time = time::time_now();
    let download_results = FirefoxDownloads::get_downloads();

    let download_data = match download_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[artemis-core] Artemis failed to get Firefox downloads: {err:?}");
            return Err(ApplicationError::FirefoxDownloads);
        }
    };

    let serde_data_result = serde_json::to_value(download_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize Firefox downloads: {err:?}");
            return Err(ApplicationError::Serialize);
        }
    };

    let output_name = "firefox_downloads";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Parse Chromium history
pub(crate) fn chromium_history(output: &mut Output, filter: &bool) -> Result<(), ApplicationError> {
    let start_time = time::time_now();

    let history_results = ChromiumHistory::get_history();
    let history_data = match history_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[artemis-core] Artemis failed to get Chromium history: {err:?}");
            return Err(ApplicationError::ChromiumHistory);
        }
    };

    let serde_data_result = serde_json::to_value(history_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize Chromium history: {err:?}");
            return Err(ApplicationError::Serialize);
        }
    };

    let output_name = "chromium_history";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

/// Parse Chromium downloads
pub(crate) fn chromium_downloads(
    output: &mut Output,
    filter: &bool,
) -> Result<(), ApplicationError> {
    let start_time = time::time_now();

    let download_results = ChromiumDownloads::get_downloads();
    let download_data = match download_results {
        Ok(results) => results,
        Err(err) => {
            warn!("[artemis-core] Artemis failed to get Chromium downloads: {err:?}");
            return Err(ApplicationError::ChromiumDownloads);
        }
    };

    let serde_data_result = serde_json::to_value(download_data);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[artemis-core] Failed to serialize Chromium downloads: {err:?}");
            return Err(ApplicationError::Serialize);
        }
    };

    let output_name = "chromium_downloads";
    output_data(&serde_data, output_name, output, &start_time, filter)
}

// Output application artifacts
pub(crate) fn output_data(
    serde_data: &Value,
    output_name: &str,
    output: &mut Output,
    start_time: &u64,
    filter: &bool,
) -> Result<(), ApplicationError> {
    if *filter {
        if let Some(script) = &output.filter_script.clone() {
            let args = vec![serde_data.to_string(), output_name.to_string()];
            if let Some(name) = &output.filter_name.clone() {
                let filter_result = filter_script(output, &args, name, script);
                return match filter_result {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        error!(
                            "[artemis-core] Could not apply filter script to application data: {err:?}"
                        );
                        Err(ApplicationError::FilterOutput)
                    }
                };
            }
            let filter_result = filter_script(output, &args, "UnknownFilterName", script);
            return match filter_result {
                Ok(_) => Ok(()),
                Err(err) => {
                    error!(
                    "[artemis-core] Could not apply unknown filter script to application data: {err:?}"
                );
                    Err(ApplicationError::FilterOutput)
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
        return Err(ApplicationError::Format);
    };
    match output_status {
        Ok(_) => {}
        Err(err) => {
            error!("[artemis-core] Could not output data: {err:?}");
            return Err(ApplicationError::Output);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::applications::artifacts::{
            chromium_downloads, chromium_history, firefox_downloads, firefox_history,
        },
        utils::artemis_toml::Output,
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
        let mut output = output_options("safari_test", "json", "./tmp", false);

        let status = safari_history(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_safari_downloads() {
        let mut output = output_options("safari_test", "json", "./tmp", false);

        let status = safari_downloads(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    #[ignore = "Requires Firefox sqlite file"]
    fn test_firefox_history() {
        let mut output = output_options("firefox_test", "json", "./tmp", false);

        let status = firefox_history(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    #[ignore = "Requires Firefox sqlite file"]
    fn test_firefox_downloads() {
        let mut output = output_options("firefox_test", "json", "./tmp", false);

        let status = firefox_downloads(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_chromium_history() {
        let mut output = output_options("chromium_test", "json", "./tmp", false);

        let status = chromium_history(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_chromium_downloads() {
        let mut output = output_options("chromium_test", "json", "./tmp", false);

        let status = chromium_downloads(&mut output, &false).unwrap();
        assert_eq!(status, ());
    }
}
