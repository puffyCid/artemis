use crate::{filestore::database::get_collections, server::ServerState};
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    Json,
};
use common::server::collections::{CollectionRequest, QuickCollection};
use log::error;
use std::net::SocketAddr;

/// Send quick collection to target endpoint
pub(crate) async fn endpoint_quick(
    State(state): State<ServerState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    Json(data): Json<QuickCollection>,
) -> Result<(), StatusCode> {
    if addr.ip().to_string() != "127.0.0.1" {
        return Err(StatusCode::FORBIDDEN);
    }
    let quick = serde_json::to_string(&data);
    if let Ok(command) = quick {
        let status = state.clients.send(command);
        if status.is_err() {
            error!(
                "[server] Could not send quick command to client {}: {:?}",
                data.target,
                status.unwrap_err()
            );
        }
        return Ok(());
    }

    error!(
        "[server] Could not serialize quick collection {data:?}: {:?}",
        quick.unwrap_err()
    );
    Err(StatusCode::INTERNAL_SERVER_ERROR)
}

/// Send list of all collection requests to WebUI
pub(crate) async fn get_collections_db(
    State(state): State<ServerState>,
) -> Result<Json<Vec<CollectionRequest>>, StatusCode> {
    let collections_result = get_collections(&state.central_collect_db).await;
    let collections = match collections_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Could not get collections: {err:?}");
            Vec::new()
        }
    };
    Ok(Json(collections))
}

#[cfg(test)]
mod tests {
    use crate::{
        frontend::collections::endpoint_quick,
        server::ServerState,
        utils::{config::read_config, filesystem::create_dirs},
    };
    use axum::{
        extract::{ConnectInfo, State},
        Json,
    };
    use common::server::collections::{CollectionType, QuickCollection};
    use redb::Database;
    use std::{
        net::{IpAddr, Ipv4Addr, SocketAddr},
        path::PathBuf,
        sync::Arc,
    };
    use tokio::sync::broadcast;

    #[tokio::test]
    async fn test_endpoint_quick() {
        let mut quick = QuickCollection {
            target: String::from("4c59c439-6dec-47dd-b087-826cdc678a10"),
            collection_type: CollectionType::Processes,
        };
        let test = Json(quick.clone());
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        create_dirs("./tmp").await.unwrap();

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let (clients, _rx) = broadcast::channel(100);
        let central_collect_db = Arc::new(
            Database::create("./tmp/collectionsquick.redb")
                .expect("Could not setup central collections redb"),
        );

        let server_state = ServerState {
            config,
            clients,
            central_collect_db,
        };
        let test2 = State(server_state);

        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);

        let _ = endpoint_quick(test2.clone(), ConnectInfo(address), test)
            .await
            .unwrap();

        quick.collection_type = CollectionType::Filelist;
        let test = Json(quick.clone());

        let _ = endpoint_quick(test2, ConnectInfo(address), test)
            .await
            .unwrap();
    }
}
