use crate::{
    artifacts::enrollment::{Endpoint, EndpointInfo},
    db::endpoints::{enroll_endpointdb, lookup_endpoint},
    server::ServerState,
    utils::uuid::generate_uuid,
};
use axum::Json;
use axum::{extract::State, http::StatusCode};
use log::error;
use redb::Database;
use serde::{Deserialize, Serialize};
use serde_json::Error;

#[derive(Debug, Deserialize)]
pub(crate) struct Enrollment {
    enroll_key: String,
    endpoint_info: EndpointInfo,
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

    let endpoint_id = generate_uuid();

    let _ = enroll_endpointdb(&data.endpoint_info, &endpoint_id, &state.endpoint_db);
    let enrolled = Enrolled {
        endpoint_id,
        client_config: String::new(),
    };

    Ok(Json(enrolled))
}

/// Verify the provided `endpoint_id` is registered with artemis. Based on path to storage directory
pub(crate) fn verify_enrollment(data: &str, ip: &str, db: &Database) -> bool {
    let verify_result: Result<Endpoint, Error> = serde_json::from_str(data);
    let verify = match verify_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to deserialize endpoint verification from {ip}: {err:?}");
            return false;
        }
    };

    let value_result = lookup_endpoint(db, &verify.endpoint_id);
    let (found, _) = match value_result {
        Ok(result) => result,
        Err(err) => {
            error!(
                "[server] Could not lookup {} in endpoints db: {err:?}",
                verify.endpoint_id
            );
            return false;
        }
    };
    found
}

#[cfg(test)]
mod tests {
    use super::verify_enrollment;
    use crate::{
        artifacts::{enrollment::EndpointInfo, systeminfo::Memory},
        db::tables::setup_db,
        enrollment::enroll::{enroll_endpoint, Enrollment},
        server::ServerState,
        utils::config::read_config,
    };
    use axum::{extract::State, Json};
    use std::path::PathBuf;

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

        let config = read_config(&test_location.display().to_string()).unwrap();
        let endpointdb = setup_db(&format!(
            "{}/endpoints.redb",
            &config.endpoint_server.storage
        ))
        .unwrap();

        let jobdb = setup_db(&format!("{}/jobs.redb", &config.endpoint_server.storage)).unwrap();

        let state_server = ServerState {
            config,
            endpoint_db: endpointdb,
            job_db: jobdb,
        };
        let test2 = State(state_server);

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

        let config = read_config(&test_location.display().to_string()).unwrap();
        let endpointdb = setup_db(&format!(
            "{}/endpoints.redb",
            &config.endpoint_server.storage
        ))
        .unwrap();

        let jobdb = setup_db(&format!("{}/jobs.redb", &config.endpoint_server.storage)).unwrap();

        let state_server = ServerState {
            config,
            endpoint_db: endpointdb,
            job_db: jobdb,
        };
        let test2 = State(state_server);

        let result = enroll_endpoint(test2, test).await.unwrap();
        assert!(!result.endpoint_id.is_empty())
    }

    #[test]
    fn test_verify_enrollment() {
        let data = r#"{"endpoint_id":"3482136c-3176-4272-9bd7-b79f025307d6","pulse":true,"timestamp":1111111,"jobs_running":0}"#;
        let ip = "127.0.0.1";

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/endpoints.redb");
        let path = test_location.display().to_string();
        let db = setup_db(&path).unwrap();

        let result = verify_enrollment(data, ip, &db);
        assert!(result)
    }
}
