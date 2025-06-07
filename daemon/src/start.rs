use crate::{
    collection::collect::CollectEndpoint,
    utils::{
        config::{Daemon, DaemonToml, ServerToml, server},
        setup::{move_server_config, setup_collection, setup_config, setup_enrollment},
    },
};
use tokio::time::{Duration, interval};

pub(crate) struct DaemonConfig {
    pub(crate) server: ServerToml,
    pub(crate) client: DaemonToml,
}
#[tokio::main]
pub async fn start_daemon(path: Option<&str>, alt_base: Option<&str>) {
    let mut server_path = "server.toml";

    if let Some(config_path) = path {
        server_path = config_path;
    }

    // Attempt to read to server TOML config file
    let server_config = match server(server_path, alt_base).await {
        Ok(result) => result,
        Err(_err) => return,
    };

    let mut config = DaemonConfig {
        server: server_config,
        client: DaemonToml {
            daemon: Daemon {
                node_key: String::new(),
                collection_path: String::new(),
                log_level: String::new(),
            },
        },
    };

    // Attempt to connect to server
    setup_enrollment(&mut config).await;
    setup_config(&mut config).await;

    // We have enough info connect to our server.
    // Can move our server.toml to our base config directory. Ex: /var/artemis/server.toml
    move_server_config(server_path, alt_base).await;
    start(&mut config).await
}

/// Continously poll our server for jobs and collections
async fn start(config: &mut DaemonConfig) {
    let max_attempts = 8;
    let mut count = 0;

    let long_pause = 300;
    let pause = 8;
    let mut pause_interval = interval(Duration::from_secs(pause));
    pause_interval.tick().await;
    let mut long_interval = interval(Duration::from_secs(long_pause));
    long_interval.tick().await;
    loop {
        if count == max_attempts {
            long_interval.tick().await;
            count = 0;
        }
        let collection = match config.collect_request().await {
            Ok(result) => result,
            Err(_err) => {
                count += 1;
                pause_interval.tick().await;
                continue;
            }
        };
        setup_collection(config, &collection).await;

        pause_interval.tick().await;
    }
}

#[cfg(test)]
mod tests {}
