use super::config::daemon;
use crate::{enrollment::enroll::EnrollEndpoint, start::DaemonConfig};
use log::error;
use std::time::Duration;
use tokio::time::sleep;

/// Enroll the endpoint to our server based on parsed Server.toml file
pub(crate) async fn setup_enrollment(config: &mut DaemonConfig) {
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
    config.client.daemon.node_key = enroll.node_key;
}

/// Setup proper config directories for the daemon
pub(crate) async fn setup_daemon(daemon_config: &mut DaemonConfig) {
    let config = match daemon(&daemon_config.client.daemon.node_key, None, None).await {
        Ok(result) => result,
        Err(err) => {
            error!("[daemon] Could not setup daemon TOML config: {err:?}");
            return;
        }
    };

    daemon_config.client = config;
}
