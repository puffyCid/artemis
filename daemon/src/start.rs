use crate::{
    enrollment::enroll::EnrollEndpoint,
    utils::config::{ServerConfig, server},
};
use log::error;
use std::time::Duration;
use tokio::time::sleep;

pub(crate) struct DaemonConfig {
    pub(crate) server: ServerConfig,
    pub(crate) client: bool,
}
#[tokio::main]
pub async fn start_daemon(path: Option<&str>) {
    let mut server_path = "server.toml";

    if let Some(config_path) = path {
        server_path = config_path;
    }

    let servr_config = match server(server_path).await {
        Ok(result) => result,
        Err(_err) => return,
    };

    let config = DaemonConfig {
        server: servr_config,
        client: false,
    };

    let mut enroll = match config.enroll_request().await {
        Ok(result) => result,
        Err(_err) => return,
    };

    let max_attempts = 8;
    let mut count = 0;

    // If we get `node_invalid` response. We have to enroll again. We attempt 8 more enrollments max
    while enroll.node_invalid && count != max_attempts {
        let pause = 6;
        // Pause for 6 seconds between each attempt
        sleep(Duration::from_secs(pause)).await;

        let enroll_attempt = match config.enroll_request().await {
            Ok(result) => result,
            Err(_err) => return,
        };

        if !enroll.node_invalid {
            enroll = enroll_attempt;
            break;
        }

        count += 1;
    }

    if enroll.node_invalid {
        error!("[daemon] Endpoint still invalid despite 8 enrollment attempts");
        return;
    }
}

#[cfg(test)]
mod tests {}
