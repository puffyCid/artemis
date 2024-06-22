use crate::{
    filestore::endpoints::create_endpoint_path, server::ServerState,
    utils::filesystem::is_directory,
};
use axum::Json;
use axum::{extract::State, http::StatusCode};
use common::server::enrollment::{EnrollSystem, EnrollmentResponse};
use log::error;

/// Enroll an endpoint
pub(crate) async fn enroll_endpoint(
    State(state): State<ServerState>,
    Json(data): Json<EnrollSystem>,
) -> Result<Json<EnrollmentResponse>, StatusCode> {
    let key = data.enroll_key;

    // Check to make sure the endpoint contains the correct enrollment key
    if key != state.config.enroll_key {
        return Err(StatusCode::BAD_REQUEST);
    }

    let id_result =
        create_endpoint_path(&state.config.endpoint_server.storage, &data.enrollment_info).await;

    let endpoint_id = match id_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Could not create enrollment storage directory: {err:?}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let enrolled = EnrollmentResponse { endpoint_id };

    Ok(Json(enrolled))
}

/// Verify the provided id is registered with artemis. Based on path to storage directory
pub(crate) fn verify_enrollment(id: &str, platform: &str, path: &str) -> Result<(), StatusCode> {
    let endpoint_path = format!("{path}/{platform}/{id}");
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
        enrollment::enroll::enroll_endpoint,
        server::ServerState,
        utils::{config::read_config, filesystem::create_dirs},
    };
    use axum::{extract::State, Json};
    use common::{
        server::enrollment::{EnrollSystem, Enrollment},
        system::Memory,
    };
    use redb::Database;
    use std::{path::PathBuf, sync::Arc};
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_enroll_endpoint() {
        let info = EnrollSystem {
            enroll_key: String::from("arandomkey"),
            enrollment_info: Enrollment {
                boot_time: 0,
                hostname: String::from("test"),
                ip: String::from("127.0.0.1"),
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
                artemis_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };
        let test = Json(info);
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        create_dirs("./tmp").await.unwrap();

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let (clients, _rx) = broadcast::channel(100);
        let central_collect_db = Arc::new(
            Database::create(format!(
                "{}/collections15.redb",
                config.endpoint_server.storage
            ))
            .expect("Could not setup central collections redb"),
        );

        let server_state = ServerState {
            config,
            clients,
            central_collect_db,
        };

        let test2 = State(server_state);

        let result = enroll_endpoint(test2, test).await.unwrap();
        assert!(!result.endpoint_id.is_empty())
    }

    #[tokio::test]
    #[should_panic(expected = "400")]
    async fn test_enroll_endpoint_bad() {
        let info = EnrollSystem {
            enroll_key: String::from("bad"),
            enrollment_info: Enrollment {
                boot_time: 0,
                hostname: String::from("test"),
                ip: String::from("127.0.0.1"),
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
                artemis_version: env!("CARGO_PKG_VERSION").to_string(),
            },
        };
        let test = Json(info);
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        create_dirs("./tmp").await.unwrap();

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let (clients, _rx) = broadcast::channel(100);
        let central_collect_db = Arc::new(
            Database::create(format!(
                "{}/collections10.redb",
                config.endpoint_server.storage
            ))
            .expect("Could not setup central collections redb"),
        );

        let server_state = ServerState {
            config,
            clients,
            central_collect_db,
        };
        let test2 = State(server_state);

        let result = enroll_endpoint(test2, test).await.unwrap();
        assert!(!result.endpoint_id.is_empty())
    }

    #[test]
    fn test_verify_enrollment() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data");
        let path = test_location.display().to_string();

        verify_enrollment("3482136c-3176-4272-9bd7-b79f025307d6", "", &path).unwrap();
    }
}
