use crate::filestore::endpoints::{get_endpoints, recent_heartbeat};
use crate::{filestore::endpoints::endpoint_count, server::ServerState};
use axum::Json;
use axum::{extract::State, http::StatusCode};
use common::server::{EndpointList, EndpointOS, EndpointRequest, Heartbeat};
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
    let storage_path = state.config.endpoint_server.storage;
    let info = data.trim().split('.').collect::<Vec<_>>();

    let min_size = 2;
    if info.len() < min_size {
        println!("[server] Did not receive enough info for endpoint lookup {data}");
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }
    let endpoint_dir = format!("{}/{}/{}", storage_path, info[0], info[1]);
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

#[cfg(test)]
mod tests {
    use crate::{
        frontend::endpoints::{endpoint_list, endpoint_stats},
        server::ServerState,
        utils::{config::read_config, filesystem::create_dirs},
    };
    use axum::{extract::State, Json};
    use common::server::{EndpointOS, EndpointRequest};
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
            pagination: String::new(),
            filter: EndpointOS::Linux,
            tags: Vec::new(),
            search: String::new(),
        });

        let result = endpoint_list(test2, data).await.unwrap();
        assert_eq!(result.0[0].hostname, "hello");
    }
}
