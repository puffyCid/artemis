use crate::{
    filestore::{
        collections::get_collection_info, database::get_collections, endpoints::glob_paths,
    },
    server::ServerState,
};
use axum::{
    extract::{ConnectInfo, State},
    http::StatusCode,
    Json,
};
use common::server::{
    collections::{CollectionInfo, CollectionRequest, CollectionTargets, QuickCollection},
    webui::CollectRequest,
};
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

/// Send list of all collection requests to `WebUI`
pub(crate) async fn get_collections_db(
    State(state): State<ServerState>,
    Json(data): Json<CollectRequest>,
) -> Result<Json<Vec<CollectionRequest>>, StatusCode> {
    let collections_result = get_collections(&state.central_collect_db, &data).await;
    let collections = match collections_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Could not get collections: {err:?}");
            Vec::new()
        }
    };
    Ok(Json(collections))
}

pub(crate) async fn get_endpoints_collection_status(
    State(state): State<ServerState>,
    Json(targets): Json<CollectionTargets>,
) -> Result<Json<Vec<CollectionInfo>>, StatusCode> {
    let glob_path = format!("{}/*/*", state.config.endpoint_server.storage);
    let glob_result = glob_paths(&glob_path);
    let paths = match glob_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Could not glob collections: {err:?}");
            return Err(StatusCode::INTERNAL_SERVER_ERROR);
        }
    };

    let mut info = Vec::new();
    let limit = 50;
    let mut count = 0;

    for target in targets.targets {
        for path in &paths {
            if !path.full_path.ends_with(&target) {
                continue;
            }

            let value = get_collection_info(&path.full_path, &targets.id).await;
            if value.is_err() {
                continue;
            }

            info.push(value.unwrap());
            break;
        }
        count += 1;
        if count == limit {
            break;
        }
    }

    Ok(Json(info))
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
