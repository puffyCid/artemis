use crate::filestore::cache::processes::process_list;
use crate::filestore::endpoints::{get_endpoints, recent_heartbeat};
use crate::{filestore::endpoints::endpoint_count, server::ServerState};
use axum::Json;
use axum::{extract::State, http::StatusCode};
use common::server::heartbeat::Heartbeat;
use common::server::jobs::ProcessJob;
use common::server::webui::{EndpointList, EndpointOS, EndpointRequest};
use log::error;

/// Count number of Endpoints based on OS type
pub(crate) async fn endpoint_stats(
    State(state): State<ServerState>,
    Json(data): Json<EndpointOS>,
) -> Result<Json<usize>, StatusCode> {
    let count_result = endpoint_count(&state.config.endpoint_server.storage, &data).await;

    let count = match count_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Could not count endpoints {data:?}: {err:?}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    Ok(Json(count))
}

/// Get a list of endpoints that have enrolled
pub(crate) async fn endpoint_list(
    State(state): State<ServerState>,
    Json(data): Json<EndpointRequest>,
) -> Result<Json<Vec<EndpointList>>, StatusCode> {
    let storage_path = state.config.endpoint_server.storage;
    let mut pattern = format!("{}/{:?}/*/enroll.json", storage_path, data.filter);

    // If Filter is all replace pattern with wildcard. Otherwise use filter
    if data.filter == EndpointOS::All {
        pattern = format!("{}/*/*/enroll.json", storage_path);
    }
    let entries_result = get_endpoints(&pattern, &data).await;
    let entries = match entries_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Could not get endpoint list: {err:?}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };
    Ok(Json(entries))
}

/// Get heartbeat info related to endpoint
pub(crate) async fn endpoint_info(
    State(state): State<ServerState>,
    data: String,
) -> Result<Json<Heartbeat>, StatusCode> {
    let endpoint_dir = endpoint_path(&data, &state).await?;
    let entries_result = recent_heartbeat(&endpoint_dir).await;
    let entry = match entries_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Could not get heartbeat info for {data}: {err:?}",);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    Ok(Json(entry))
}

/// Get process listing for endpoint
pub(crate) async fn endpoint_processes(
    State(state): State<ServerState>,
    data: String,
) -> Result<Json<ProcessJob>, StatusCode> {
    let endpoint_dir = endpoint_path(&data, &state).await?;
    let entries_result = process_list(&endpoint_dir).await;
    let entry = match entries_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Could not get heartbeat info for {data}: {err:?}",);
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    Ok(Json(entry))
}

/// Path to endpoint storage directory
async fn endpoint_path(data: &str, state: &ServerState) -> Result<String, StatusCode> {
    let storage_path = &state.config.endpoint_server.storage;
    let info = data.trim().split('.').collect::<Vec<_>>();

    let min_size = 2;
    if info.len() < min_size {
        error!("[server] Did not receive enough info for endpoint lookup {data}");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let endpoint_dir = format!("{}/{}/{}", storage_path, info[0], info[1]);
    Ok(endpoint_dir)
}

#[cfg(test)]
mod tests {
    use crate::{
        enrollment::enroll::enroll_endpoint,
        frontend::endpoints::{
            endpoint_info, endpoint_list, endpoint_path, endpoint_processes, endpoint_stats,
        },
        server::ServerState,
        utils::{config::read_config, filesystem::create_dirs},
    };
    use axum::{extract::State, Json};
    use common::server::enrollment::{EnrollSystem, Enrollment};
    use common::{
        server::webui::{EndpointOS, EndpointRequest},
        system::Memory,
    };
    use std::{collections::HashMap, path::PathBuf, sync::Arc};
    use tokio::sync::RwLock;

    #[tokio::test]
    async fn test_endpoint_stats() {
        let test = Json(EndpointOS::Windows);
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        create_dirs("./tmp").await.unwrap();

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let command = Arc::new(RwLock::new(HashMap::new()));
        let server_state = ServerState { config, command };
        let test2 = State(server_state);

        let _ = endpoint_stats(test2, test).await.unwrap();
    }

    #[tokio::test]
    async fn test_endpoint_list() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        create_dirs("./tmp").await.unwrap();

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let command = Arc::new(RwLock::new(HashMap::new()));
        let server_state = ServerState { config, command };
        let test2 = State(server_state);

        let data = Json(EndpointRequest {
            offset: 0,
            filter: EndpointOS::MacOS,
            tags: Vec::new(),
            search: String::new(),
            count: 0,
        });

        let info = EnrollSystem {
            enroll_key: String::from("arandomkey"),
            enrollment_info: Enrollment {
                boot_time: 0,
                hostname: String::from("hello"),
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
            },
        };

        let test = Json(info);
        let _ = enroll_endpoint(test2.clone(), test).await.unwrap();

        let _ = endpoint_list(test2, data).await.unwrap();
    }

    #[tokio::test]
    async fn test_endpoint_path() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        create_dirs("./tmp/Darwin/123").await.unwrap();

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let command = Arc::new(RwLock::new(HashMap::new()));
        let server_state = ServerState { config, command };

        let result = endpoint_path("Darwin.123", &server_state).await.unwrap();
        assert!(result.contains("123"))
    }

    #[tokio::test]
    #[should_panic(expected = "500")]
    async fn test_endpoint_info() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        create_dirs("./tmp/Darwin/123").await.unwrap();

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let command = Arc::new(RwLock::new(HashMap::new()));
        let server_state = ServerState { config, command };
        let test2 = State(server_state);

        let _ = endpoint_info(test2, "Darwin.123".to_string())
            .await
            .unwrap();
    }

    #[tokio::test]
    #[should_panic(expected = "500")]
    async fn test_endpoint_processes() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        create_dirs("./tmp").await.unwrap();

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let command = Arc::new(RwLock::new(HashMap::new()));
        let server_state = ServerState { config, command };
        let test2 = State(server_state);

        let _ = endpoint_processes(test2, "Darwin.123".to_string())
            .await
            .unwrap();
    }
}
