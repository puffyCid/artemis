use super::error::EnrollError;
use crate::{start::DaemonConfig, utils::info::get_info};
use common::system::SystemInfo;
use log::error;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Deserialize, Debug)]
pub(crate) struct EnrollResponse {
    /// Unique key for our endpoint
    pub(crate) endpoint_id: String,
    /// If invalid we should enroll again
    pub(crate) endpoint_invalid: bool,
}

#[derive(Serialize, Debug)]
pub(crate) struct EnrollRequest {
    /// Enrollment key for the server
    enroll_key: String,
    /// UUID for our endpoint
    endpoint_uuid: String,
    /// Simple endpoint info
    info: SystemInfo,
}

#[derive(Deserialize, Debug)]
pub(crate) struct BadRequest {
    /// Server message
    pub(crate) message: String,
}

pub(crate) trait EnrollEndpoint {
    async fn enroll_request(&self) -> Result<EnrollResponse, EnrollError>;
}

impl EnrollEndpoint for DaemonConfig {
    /// Send the enrollment request to our server
    async fn enroll_request(&self) -> Result<EnrollResponse, EnrollError> {
        let url = format!(
            "{}:{}/v{}/{}",
            self.server.server.url,
            self.server.server.port,
            self.server.server.version,
            self.server.server.enrollment
        );

        let info = get_info();
        let enroll = EnrollRequest {
            enroll_key: self.server.server.key.clone(),
            endpoint_uuid: Uuid::new_v4().hyphenated().to_string(),
            info,
        };

        let client = Client::new();
        let mut builder = client.post(&url).json(&enroll);
        builder = builder.header("accept", "application/json");

        let res = match builder.send().await {
            Ok(result) => result,
            Err(err) => {
                error!("[daemon] Failed to enroll endpoint: {err:?}");
                return Err(EnrollError::FailedEnrollment);
            }
        };

        if res.status() == StatusCode::BAD_REQUEST {
            let message = bad_request(&res.bytes().await.unwrap_or_default());
            error!("[daemon] Enrollment request was bad: {}", message.message);
            return Err(EnrollError::BadEnrollment);
        }

        if res.status() != StatusCode::OK {
            error!("[daemon] Got non-Ok response");
            return Err(EnrollError::EnrollmentNotOk);
        }

        let bytes = match res.bytes().await {
            Ok(result) => result,
            Err(err) => {
                error!("[daemon] Failed to get enroll bytes: {err:?}");
                return Err(EnrollError::FailedEnrollment);
            }
        };

        let enroll_key: EnrollResponse = match serde_json::from_slice(&bytes) {
            Ok(result) => result,
            Err(err) => {
                error!("[daemon] Failed to serialize enroll response: {err:?}");
                return Err(EnrollError::FailedEnrollment);
            }
        };

        Ok(enroll_key)
    }
}

/// Process 400 response code
pub(crate) fn bad_request(bytes: &[u8]) -> BadRequest {
    let message: BadRequest = match serde_json::from_slice(bytes) {
        Ok(result) => result,
        Err(err) => {
            error!("[daemon] Failed to deserialize bad request (400) message: {err:?}");
            return BadRequest {
                message: format!("Failed to deserialize bad request (400) message: {err:?}"),
            };
        }
    };

    message
}

#[cfg(test)]
mod tests {
    use crate::{
        enrollment::enroll::EnrollEndpoint,
        start::DaemonConfig,
        utils::config::{Daemon, DaemonToml, server},
    };
    use httpmock::{Method::POST, MockServer};
    use serde_json::json;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_enroll_request() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let mock_server = MockServer::start();
        let port = mock_server.port();

        let mock_me = mock_server.mock(|when, then| {
            when.method(POST)
                .path("/v1/endpoint/enroll")
                .body_contains("my key")
                .body_contains("endpoint_uuid");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({ "endpoint_id": "server uuid", "endpoint_invalid": false }));
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

        let status = config.enroll_request().await.unwrap();
        mock_me.assert();

        assert_eq!(status.endpoint_id, "server uuid");
        assert!(!status.endpoint_invalid);
    }

    #[tokio::test]
    #[should_panic(expected = "BadEnrollment")]
    async fn test_enroll_bad_enrollment() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let mock_server = MockServer::start();
        let port = mock_server.port();

        let mock_me = mock_server.mock(|when, then| {
            when.method(POST)
                .path("/v1/endpoint/enroll")
                .body_contains("my key")
                .body_contains("endpoint_uuid");
            then.status(400)
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

        let _ = config.enroll_request().await.unwrap();
        mock_me.assert();
    }

    #[tokio::test]
    #[should_panic(expected = "FailedEnrollment")]
    async fn test_enroll_bad_response() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let mock_server = MockServer::start();
        let port = mock_server.port();

        let mock_me = mock_server.mock(|when, then| {
            when.method(POST)
                .path("/v1/endpoint/enroll")
                .body_contains("my key")
                .body_contains("endpoint_uuid");
            then.status(200)
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

        let _ = config.enroll_request().await.unwrap();
        mock_me.assert();
    }

    #[tokio::test]
    #[should_panic(expected = "EnrollmentNotOk")]
    async fn test_enroll_not_ok() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let mock_server = MockServer::start();
        let port = mock_server.port();

        let mock_me = mock_server.mock(|when, then| {
            when.method(POST)
                .path("/v1/endpoint/enroll")
                .body_contains("my key")
                .body_contains("endpoint_uuid");
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

        let _ = config.enroll_request().await.unwrap();
        mock_me.assert();
    }

    #[tokio::test]
    async fn test_enroll_endpoint_invalid() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let mock_server = MockServer::start();
        let port = mock_server.port();

        let mock_me = mock_server.mock(|when, then| {
            when.method(POST)
                .path("/v1/endpoint/enroll")
                .body_contains("my key")
                .body_contains("endpoint_uuid");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({ "endpoint_id": "server uuid", "endpoint_invalid": true }));
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

        let key = config.enroll_request().await.unwrap();
        assert!(key.endpoint_invalid);
        if key.endpoint_invalid {
            let _ = config.enroll_request().await.unwrap();
        }

        mock_me.assert_hits(2);
    }
}
