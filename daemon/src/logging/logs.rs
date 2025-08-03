use super::error::LoggingError;
use crate::{enrollment::enroll::bad_request, start::DaemonConfig};
use log::error;
use reqwest::{StatusCode, blocking::Client};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{BufRead, BufReader, Lines},
};

#[derive(Deserialize, Debug)]
pub(crate) struct LoggingResponse {
    /// If invalid we should enroll again
    pub(crate) endpoint_invalid: bool,
}

#[derive(Serialize, Debug)]
pub(crate) struct LoggingRequest {
    endpoint_id: String,
    logs: Vec<String>,
}

pub(crate) trait LoggingEndpoint {
    /// Send logs to server for daemon
    fn log_upload(&self) -> Result<LoggingResponse, LoggingError>;
}

impl LoggingEndpoint for DaemonConfig {
    fn log_upload(&self) -> Result<LoggingResponse, LoggingError> {
        let url = format!(
            "{}:{}/v{}/{}",
            self.server.server.url,
            self.server.server.port,
            self.server.server.version,
            self.server.server.logging
        );
        let log_path = format!("{}/daemon.log", self.server.log_path);
        let mut lines = read_log(&log_path)?;

        let client = Client::new();

        let mut messages = Vec::new();

        let limit = 200;
        while let Some(Ok(line)) = lines.next() {
            messages.push(line);

            if messages.len() == limit {
                let log_request = LoggingRequest {
                    endpoint_id: self.client.daemon.endpoint_id.clone(),
                    logs: messages,
                };

                let mut builder = client.post(&url).json(&log_request);
                builder = builder.header("accept", "application/json");
                let res = match builder.send() {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[daemon] Failed to send request for log upload: {err:?}");
                        return Err(LoggingError::FailedUpload);
                    }
                };
                if res.status() == StatusCode::BAD_REQUEST {
                    let message = bad_request(&res.bytes().unwrap_or_default());
                    error!("[daemon] Log request was bad: {}", message.message);
                    return Err(LoggingError::FailedUpload);
                }

                if res.status() != StatusCode::OK {
                    error!("[daemon] Got non-Ok logging response");
                    return Err(LoggingError::UploadNotOk);
                }

                let bytes = match res.bytes() {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[daemon] Failed to get config bytes: {err:?}");
                        return Err(LoggingError::FailedUpload);
                    }
                };

                if let Err(err) = serde_json::from_slice::<LoggingResponse>(&bytes) {
                    error!("[daemon] Failed to serialize config response: {err:?}");
                    return Err(LoggingError::UploadBadResponse);
                }

                messages = Vec::new();
            }
        }

        // Send any remaining logs event if messages is empty
        let log_request = LoggingRequest {
            endpoint_id: self.client.daemon.endpoint_id.clone(),
            logs: messages,
        };

        let mut builder = client.post(&url).json(&log_request);
        builder = builder.header("accept", "application/json");
        let res = match builder.send() {
            Ok(result) => result,
            Err(err) => {
                error!("[daemon] Failed to send request for log upload: {err:?}");
                return Err(LoggingError::FailedUpload);
            }
        };
        if res.status() == StatusCode::BAD_REQUEST {
            let message = bad_request(&res.bytes().unwrap_or_default());
            error!("[daemon] Log request was bad: {}", message.message);
            return Err(LoggingError::FailedUpload);
        }

        if res.status() != StatusCode::OK {
            error!("[daemon] Got non-Ok logging response");
            return Err(LoggingError::UploadNotOk);
        }

        let bytes = match res.bytes() {
            Ok(result) => result,
            Err(err) => {
                error!("[daemon] Failed to get config bytes: {err:?}");
                return Err(LoggingError::FailedUpload);
            }
        };

        let log_response = match serde_json::from_slice(&bytes) {
            Ok(result) => result,
            Err(err) => {
                error!("[daemon] Failed to serialize config response: {err:?}");
                return Err(LoggingError::UploadBadResponse);
            }
        };

        // Upload was successful. We clear the log to prevent duplicate entries from getting uploaded again
        clear_log(&log_path)?;

        Ok(log_response)
    }
}

/// Read the daemon.log file
fn read_log(path: &str) -> Result<Lines<BufReader<File>>, LoggingError> {
    let reader = match File::open(path) {
        Ok(result) => result,
        Err(err) => {
            error!("[daemon] Failed to open file {path}: {err:?}");
            return Err(LoggingError::OpenFile);
        }
    };
    let buf_reader = BufReader::new(reader);
    let lines = buf_reader.lines();

    Ok(lines)
}

/// When we upload log data to the server. We can clear the log
fn clear_log(path: &str) -> Result<(), LoggingError> {
    if let Err(err) = File::create(path) {
        error!("[daemon] Failed to clear log file {path}: {err:?}");
        return Err(LoggingError::ClearLog);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        logging::logs::{LoggingEndpoint, clear_log, read_log},
        start::DaemonConfig,
        utils::config::{Daemon, DaemonToml, server},
    };
    use httpmock::{Method::POST, MockServer};
    use log::{LevelFilter, error, warn};
    use serde_json::json;
    use simplelog::{Config, WriteLogger};
    use std::{
        fs::{File, create_dir_all},
        path::PathBuf,
    };

    #[test]
    fn test_log_upload() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let mock_server = MockServer::start();
        let port = mock_server.port();

        let mock_me = mock_server.mock(|when, then| {
            when.method(POST).path("/v1/endpoint/logging");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({"endpoint_invalid": false }));
        });

        let server_config = server(test_location.to_str().unwrap(), Some("./tmp/artemis")).unwrap();
        let mut config = DaemonConfig {
            server: server_config,
            client: DaemonToml {
                daemon: Daemon {
                    endpoint_id: String::from("uuid key"),
                    collection_path: String::from("/var/artemis/collections"),
                    log_level: String::from("warn"),
                },
            },
        };
        config.server.server.port = port;
        error!("my fake error");

        let status = config.log_upload().unwrap();
        mock_me.assert();

        assert_eq!(status.endpoint_invalid, false);
    }

    #[test]
    fn test_read_log() {
        create_dir_all("./tmp/artemis").unwrap();
        let log_file = File::create("./tmp/artemis/daemon2.log").unwrap();
        let _ = WriteLogger::init(LevelFilter::Warn, Config::default(), log_file);
        warn!("test warning");

        let _ = read_log("./tmp/artemis/daemon2.log").unwrap();
    }

    #[test]
    #[should_panic(expected = "ClearLog")]
    fn test_clear_log() {
        clear_log("./tmp/artemis/asdfasf/daemon.log").unwrap();
    }

    #[test]
    #[should_panic(expected = "FailedUpload")]
    fn test_log_upload_non_ok_status() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let mock_server = MockServer::start();
        let port = mock_server.port();

        let mock_me = mock_server.mock(|when, then| {
            when.method(POST)
                .path("/v1/endpoint/logging")
                .body_contains("uuid key");
            then.status(400)
                .header("content-type", "application/json")
                .body("bad response");
        });

        let server_config = server(test_location.to_str().unwrap(), Some("./tmp/artemis")).unwrap();
        let mut config = DaemonConfig {
            server: server_config,
            client: DaemonToml {
                daemon: Daemon {
                    endpoint_id: String::from("uuid key"),
                    collection_path: String::from("/var/artemis/collections"),
                    log_level: String::from("warn"),
                },
            },
        };
        config.server.server.port = port;

        let _ = config.log_upload().unwrap();
        mock_me.assert();
    }

    #[test]
    #[should_panic(expected = "UploadBadResponse")]
    fn test_log_upload_bad_response() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let mock_server = MockServer::start();
        let port = mock_server.port();

        let mock_me = mock_server.mock(|when, then| {
            when.method(POST)
                .path("/v1/endpoint/logging")
                .body_contains("uuid key");
            then.status(200)
                .header("content-type", "application/json")
                .body("bad response");
        });

        let server_config = server(test_location.to_str().unwrap(), Some("./tmp/artemis")).unwrap();
        let mut config = DaemonConfig {
            server: server_config,
            client: DaemonToml {
                daemon: Daemon {
                    endpoint_id: String::from("uuid key"),
                    collection_path: String::from("/var/artemis/collections"),
                    log_level: String::from("warn"),
                },
            },
        };
        config.server.server.port = port;

        let _ = config.log_upload().unwrap();
        mock_me.assert();
    }
}
