use crate::{
    routes,
    utils::{config::read_config, filesystem::create_dirs},
};
use common::server::config::ArtemisConfig;
use log::error;
use redb::Database;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::sync::broadcast;

#[derive(Debug, Clone)]
pub(crate) struct ServerState {
    pub(crate) config: ArtemisConfig,
    pub(crate) clients: broadcast::Sender<String>,
    pub(crate) central_collect_db: Arc<Database>,
}

#[tokio::main]
pub async fn start(path: &str) {
    let config_result = read_config(path).await;
    let config = match config_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Could not read config at {path}. Cannot start server without a config file: {err:?}");
            return;
        }
    };

    let dir_result = create_dirs(&config.endpoint_server.storage).await;
    if dir_result.is_err() {
        error!("[server] Failed to start artemis server. Could not create storage directory",);
        return;
    }

    let (clients, _rx) = broadcast::channel(100);
    let central_collect_db = Arc::new(
        Database::create(format!(
            "{}/collections.redb",
            config.endpoint_server.storage
        ))
        .expect("Could not setup central collections redb"),
    );

    let server_state = ServerState {
        config,
        clients,
        central_collect_db,
    };

    let app = routes::setup_routes().with_state(server_state);
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);

    let listen = tokio::net::TcpListener::bind(&address).await;
    if listen.is_err() {
        error!(
            "[server] Failed to start artemis server. Could not listen on IP and port {:?}",
            listen.unwrap_err()
        );
        return;
    }
    let status = axum::serve(
        listen.unwrap(),
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await;

    if status.is_err() {
        error!(
            "[server] Failed to start artemis server: {:?}",
            status.unwrap_err()
        );
    }
}

#[cfg(test)]
mod tests {
    use super::start;
    use std::path::PathBuf;

    #[test]
    #[ignore = "Spawns server"]
    fn test_start() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        let config_path = test_location.display().to_string();
        start(&config_path)
    }
}
