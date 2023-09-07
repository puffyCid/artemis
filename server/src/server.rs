use crate::{
    db::tables::setup_db,
    routes,
    utils::{
        config::{read_config, ArtemisConfig},
        filesystem::create_dirs,
    },
};
use log::error;
use redb::Database;
use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};

#[derive(Debug, Clone)]
pub(crate) struct ServerState {
    pub(crate) config: ArtemisConfig,
    pub(crate) endpoint_db: Arc<Database>,
    pub(crate) job_db: Arc<Database>,
}

#[tokio::main]
async fn start(path: &str) {
    let config_result = read_config(path);
    let config = match config_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Could not read config at {path}. Cannot start server without a config file: {err:?}");
            return;
        }
    };

    let dir_result = create_dirs(&config.endpoint_server.storage);
    if dir_result.is_err() {
        error!("[server] Failed to start artemis server. Could not create storage directory",);
        return;
    }

    let endpoint_db = setup_state(&format!(
        "{}/endpoints.redb",
        config.endpoint_server.storage
    ));
    let job_db = setup_state(&format!("{}/jobs.redb", config.endpoint_server.storage));

    let server_state = ServerState {
        config,
        endpoint_db,
        job_db,
    };

    let app = routes::setup_routes().with_state(server_state);
    let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);

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

/// Setup the server state for `axum::State`. If we cannot setup the databases then we cannot start the server
fn setup_state(path: &str) -> Arc<Database> {
    setup_db(path)
        .unwrap_or_else(|_| unreachable!("Could not setup database at {path}. Cannot start server"))
}

#[cfg(test)]
mod tests {
    use super::{setup_state, start};
    use std::path::PathBuf;

    #[test]
    #[ignore = "Spawns server"]
    fn test_start() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        let config_path = test_location.display().to_string();
        start(&config_path)
    }

    #[test]
    fn test_setup_state() {
        let _ = setup_state("./tmp/endpoints.redb");
    }
}
