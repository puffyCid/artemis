use super::{
    config::{DaemonToml, daemon},
    encoding::base64_decode_standard,
    env::get_env_value,
    info::{PlatformType, get_platform_enum},
};
use crate::{
    collection::collect::CollectResponse, configuration::config::ConfigEndpoint,
    enrollment::enroll::EnrollEndpoint, start::DaemonConfig,
};
use log::error;
use std::str::from_utf8;
use tokio::{
    fs::rename,
    time::{Duration, interval},
};

/// Enroll the endpoint to our server based on parsed Server.toml file
pub(crate) async fn setup_enrollment(config: &mut DaemonConfig) {
    let mut enroll = match config.enroll_request().await {
        Ok(result) => result,
        Err(_err) => return,
    };

    let max_attempts = 8;
    let mut count = 0;
    let pause = 6;
    // Pause for 6 seconds between each attempt
    let mut interval = interval(Duration::from_secs(pause));
    interval.tick().await;

    // If we get `node_invalid` response. We have to enroll again. We attempt 8 more enrollments max
    while enroll.node_invalid && count != max_attempts {
        interval.tick().await;

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

/// Process our collection request
pub(crate) async fn setup_collection(config: &mut DaemonConfig, collect: &CollectResponse) {
    if collect.node_invalid {
        setup_enrollment(config).await;
    }
    let collection_bytes = match base64_decode_standard(&collect.collection) {
        Ok(result) => result,
        Err(err) => {
            error!("[daemon] Could not decode TOML collection {err:?}");
            return;
        }
    };

    if let Err(err) = forensics::core::parse_toml_data(&collection_bytes) {
        error!("[daemon] Could not process TOML collection {err:?}");
    }
}

/// Get a daemon configuration from our server. If none is provided we will generate a default config
pub(crate) async fn setup_config(config: &mut DaemonConfig) {
    let daemon_config = match config.config_request().await {
        Ok(result) => result,
        Err(_err) => return setup_daemon(config).await,
    };

    // Check if we got a node_invalid response
    if daemon_config.node_invalid {
        setup_enrollment(config).await;
    }

    let toml_bytes = match base64_decode_standard(&daemon_config.config) {
        Ok(result) => result,
        Err(err) => {
            error!("[daemon] Could not decode daemon config: {err:?}. Will use default config");
            return setup_daemon(config).await;
        }
    };

    let toml_config: DaemonToml = match toml::from_str(from_utf8(&toml_bytes).unwrap_or_default()) {
        Ok(result) => result,
        Err(err) => {
            error!("[daemon] Could not parse toml daemon config: {err:?}. Will use default config");
            return setup_daemon(config).await;
        }
    };

    config.client = toml_config;
    setup_daemon(config).await;
}

/// Move our server.toml file to our base config directory. Ex: /var/artemis/server.toml
pub(crate) async fn move_server_config(path: &str, alt_artemis_path: Option<&str>) {
    let mut artemis_path = String::from("/var/artemis");

    if get_platform_enum() == PlatformType::Windows {
        let programdata = get_env_value("ProgramData");
        if programdata.is_empty() && alt_artemis_path.is_none() {
            error!(
                "[daemon] Failed to find ProgramData env value and alt path is none. Cannot move server config"
            );
            return;
        }

        artemis_path = format!("{programdata}\\artemis");
    }

    if let Err(status) = rename(path, format!("{artemis_path}/server.toml")).await {
        error!("[daemon] Could not move server.toml file to {artemis_path}: {status:?}");
    }
}

/// Setup default config directories for the daemon
async fn setup_daemon(daemon_config: &mut DaemonConfig) {
    match daemon(&mut daemon_config.client, None).await {
        Ok(_result) => {}
        Err(err) => {
            error!("[daemon] Could not setup daemon TOML config: {err:?}");
        }
    }
}
