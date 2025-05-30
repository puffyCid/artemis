use crate::utils::{
    config::{Daemon, DaemonToml, ServerToml, server},
    setup::{move_server_config, setup_config, setup_enrollment},
};

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
}

#[cfg(test)]
mod tests {}
