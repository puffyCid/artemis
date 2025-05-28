use crate::utils::{
    config::{Daemon, DaemonToml, ServerToml, server},
    setup::{setup_daemon, setup_enrollment},
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

    setup_enrollment(&mut config).await;
    setup_daemon(&mut config).await;
}

#[cfg(test)]
mod tests {}
