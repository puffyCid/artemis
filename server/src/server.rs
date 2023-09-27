use crate::{
    routes,
    utils::{
        config::{read_config, ArtemisConfig},
        filesystem::create_dirs,
    },
};
use axum::extract::ws::Message;
use log::error;
use std::{
    collections::HashMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::sync::{mpsc, RwLock};

#[derive(Debug, Clone)]
pub(crate) struct ServerState {
    pub(crate) config: ArtemisConfig,
    pub(crate) command: Arc<RwLock<HashMap<String, mpsc::Sender<Message>>>>,
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

    let command = Arc::new(RwLock::new(HashMap::new()));
    let server_state = ServerState { config, command };

    let app = routes::setup_routes().with_state(server_state);
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 8000);

    let status = axum::Server::bind(&address)
        .serve(app.into_make_service_with_connect_info::<SocketAddr>())
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
