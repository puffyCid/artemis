use crate::{
    artifacts::enrollment::{Endpoint, EndpointInfo},
    filestore::endpoints::create_endpoint_path,
    server::ServerState,
    utils::filesystem::is_directory,
};
use axum::Json;
use axum::{extract::State, http::StatusCode};
use log::error;
use serde::{Deserialize, Serialize};
use serde_json::Error;

#[derive(Debug, Deserialize)]
pub(crate) struct Enrollment {
    pub(crate) enroll_key: String,
    pub(crate) endpoint_info: EndpointInfo,
}

#[derive(Debug, Serialize)]
pub(crate) struct Enrolled {
    pub(crate) endpoint_id: String,
    //Base64 TOML client config */
    pub(crate) client_config: String,
}

/// Enroll an endpoint
pub(crate) async fn enroll_endpoint(
    State(state): State<ServerState>,
    Json(data): Json<Enrollment>,
) -> Result<Json<Enrolled>, StatusCode> {
    let key = data.enroll_key;

    // Check to make sure the endpoint contains the correct enrollment key
    if key != state.config.enroll_key {
        return Err(StatusCode::BAD_REQUEST);
    }

    let id_result =
        create_endpoint_path(&state.config.endpoint_server.storage, &data.endpoint_info).await;

    let endpoint_id = match id_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Could not create enrollment storage directory: {err:?}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let enrolled = Enrolled {
        endpoint_id,
        client_config: String::new(),
    };

    Ok(Json(enrolled))
}

/// Verify the provided `endpoint_id` is registered with artemis. Based on path to storage directory
pub(crate) fn verify_enrollment(data: &str, ip: &str, path: &str) -> Result<(), StatusCode> {
    let verify_result: Result<Endpoint, Error> = serde_json::from_str(data);
    let verify = match verify_result {
        Ok(result) => result,
        Err(err) => {
            println!("[server] Failed to deserialize endpoint verification from {ip}: {err:?}");
            return Err(StatusCode::BAD_REQUEST);
        }
    };

    let endpoint_path = format!("{path}/{}/{}", verify.platform, verify.endpoint_id);
    let status = is_directory(&endpoint_path);
    if !status {
        return Err(StatusCode::UNPROCESSABLE_ENTITY);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::verify_enrollment;
    use crate::{
        artifacts::enrollment::EndpointInfo,
        enrollment::enroll::{enroll_endpoint, Enrollment},
        server::ServerState,
        utils::{config::read_config, filesystem::create_dirs},
    };
    use axum::{extract::State, Json};
    use common::system::Memory;
    use std::{collections::HashMap, path::PathBuf, sync::Arc};
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_enroll_endpoint() {
        let info = Enrollment {
            enroll_key: String::from("arandomkey"),
            endpoint_info: EndpointInfo {
                boot_time: 0,
                hostname: String::from("test"),
                os_version: String::from("test"),
                uptime: 1,
                kernel_version: String::from("1.1"),
                platform: String::from("darwin"),
                cpu: Vec::new(),
                disks: Vec::new(),
                memory: Memory {
                    available_memory: 12,
                    free_memory: 12,
                    free_swap: 12,
                    total_memory: 12,
                    total_swap: 12,
                    used_memory: 12,
                    used_swap: 12,
                },
            },
        };
        let test = Json(info);
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        create_dirs("./tmp").await.unwrap();

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let command = Arc::new(RwLock::new(HashMap::new()));
        let server_state = ServerState { config, command };
        let test2 = State(server_state);

        let result = enroll_endpoint(test2, test).await.unwrap();
        assert!(!result.endpoint_id.is_empty())
    }

    #[tokio::test]
    #[should_panic(expected = "400")]
    async fn test_enroll_endpoint_bad() {
        let info = Enrollment {
            enroll_key: String::from("bad"),
            endpoint_info: EndpointInfo {
                boot_time: 0,
                hostname: String::from("test"),
                os_version: String::from("test"),
                uptime: 1,
                kernel_version: String::from("1.1"),
                platform: String::from("darwin"),
                cpu: Vec::new(),
                disks: Vec::new(),
                memory: Memory {
                    available_memory: 12,
                    free_memory: 12,
                    free_swap: 12,
                    total_memory: 12,
                    total_swap: 12,
                    used_memory: 12,
                    used_swap: 12,
                },
            },
        };
        let test = Json(info);
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        create_dirs("./tmp").await.unwrap();

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let command = Arc::new(RwLock::new(HashMap::new()));
        let server_state = ServerState { config, command };
        let test2 = State(server_state);

        let result = enroll_endpoint(test2, test).await.unwrap();
        assert!(!result.endpoint_id.is_empty())
    }

    #[test]
    fn test_verify_enrollment() {
        let data = r#"{"endpoint_id":"3482136c-3176-4272-9bd7-b79f025307d6","pulse":true,"timestamp":1111111,"jobs_running":0,"platform": ""}"#;
        let ip = "127.0.0.1";

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data");
        let path = test_location.display().to_string();

        verify_enrollment(data, ip, &path).unwrap();
    }
}
