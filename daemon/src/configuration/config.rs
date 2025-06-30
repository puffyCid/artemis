use super::error::ConfigError;
use crate::{enrollment::enroll::bad_request, start::DaemonConfig};
use log::error;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub(crate) struct ConfigResponse {
    /// Base64 toml endpoint config
    pub(crate) config: String,
    /// If invalid we should enroll again
    pub(crate) endpoint_invalid: bool,
}

#[derive(Serialize, Debug)]
pub(crate) struct ConfigRequest {
    /// Unique endpoint ID that was provided from the server upon enrollment
    endpoint_id: String,
}

pub(crate) trait ConfigEndpoint {
    async fn config_request(&self) -> Result<ConfigResponse, ConfigError>;
}

impl ConfigEndpoint for DaemonConfig {
    /// Send request to server for a daemon configuration
    async fn config_request(&self) -> Result<ConfigResponse, ConfigError> {
        let url = format!(
            "{}:{}/v{}/{}",
            self.server.server.url,
            self.server.server.port,
            self.server.server.version,
            self.server.server.config
        );

        let config_req = ConfigRequest {
            endpoint_id: self.client.daemon.endpoint_id.clone(),
        };

        let client = Client::new();
        let mut builder = client.post(&url).json(&config_req);
        builder = builder.header("accept", "application/json");
        let res = match builder.send().await {
            Ok(result) => result,
            Err(err) => {
                error!("[daemon] Failed to send request for config: {err:?}");
                return Err(ConfigError::FailedConfig);
            }
        };
        if res.status() == StatusCode::BAD_REQUEST {
            let message = bad_request(&res.bytes().await.unwrap_or_default());
            error!("[daemon] Config request was bad: {}", message.message);
            return Err(ConfigError::BadConfig);
        }

        if res.status() != StatusCode::OK {
            error!("[daemon] Got non-Ok response");
            return Err(ConfigError::ConfigNotOk);
        }

        let bytes = match res.bytes().await {
            Ok(result) => result,
            Err(err) => {
                error!("[daemon] Failed to get config bytes: {err:?}");
                return Err(ConfigError::FailedConfig);
            }
        };

        let config_data: ConfigResponse = match serde_json::from_slice(&bytes) {
            Ok(result) => result,
            Err(err) => {
                error!("[daemon] Failed to serialize config response: {err:?}");
                return Err(ConfigError::FailedConfig);
            }
        };

        Ok(config_data)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        configuration::config::ConfigEndpoint,
        start::DaemonConfig,
        utils::config::{Daemon, DaemonToml, server},
    };
    use httpmock::{Method::POST, MockServer};
    use serde_json::json;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_config_request() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let mock_server = MockServer::start();
        let port = mock_server.port();

        let mock_me = mock_server.mock(|when, then| {
            when.method(POST)
                .path("/v1/endpoint/config")
                .body_contains("uuid key");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({ "config": "base64 blob", "endpoint_invalid": false }));
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

        let status = config.config_request().await.unwrap();
        mock_me.assert();

        assert_eq!(status.config, "base64 blob");
        assert_eq!(status.endpoint_invalid, false);
    }

    #[tokio::test]
    #[should_panic(expected = "BadConfig")]
    async fn test_config_bad_enrollment() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let mock_server = MockServer::start();
        let port = mock_server.port();

        let mock_me = mock_server.mock(|when, then| {
            when.method(POST)
                .path("/v1/endpoint/config")
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

        let _ = config.config_request().await.unwrap();
        mock_me.assert();
    }

    #[tokio::test]
    #[should_panic(expected = "FailedConfig")]
    async fn test_config_bad_response() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let mock_server = MockServer::start();
        let port = mock_server.port();

        let mock_me = mock_server.mock(|when, then| {
            when.method(POST)
                .path("/v1/endpoint/config")
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

        let _ = config.config_request().await.unwrap();
        mock_me.assert();
    }

    #[tokio::test]
    #[should_panic(expected = "ConfigNotOk")]
    async fn test_config_not_ok() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let mock_server = MockServer::start();
        let port = mock_server.port();

        let mock_me = mock_server.mock(|when, then| {
            when.method(POST)
                .path("/v1/endpoint/config")
                .body_contains("my key")
                .body_contains("endpoint_id");
            then.status(500)
                .header("content-type", "application/json")
                .body("bad response");
        });

        let server_config = server(test_location.to_str().unwrap(), Some("./tmp/artemis")).unwrap();
        let mut config = DaemonConfig {
            server: server_config,
            client: DaemonToml {
                daemon: Daemon {
                    endpoint_id: String::new(),
                    collection_path: String::from("/var/artemis/collections"),
                    log_level: String::from("warn"),
                },
            },
        };
        config.server.server.port = port;

        let _ = config.config_request().await.unwrap();
        mock_me.assert();
    }
}
