use crate::{filestore::endpoints::endpoint_count, server::ServerState};
use axum::Json;
use axum::{extract::State, http::StatusCode};
use common::server::EndpointOS;
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

#[cfg(test)]
mod tests {
    use crate::{
        frontend::endpoints::endpoint_stats,
        server::ServerState,
        utils::{config::read_config, filesystem::create_dirs},
    };
    use axum::{extract::State, Json};
    use common::server::EndpointOS;
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
}
