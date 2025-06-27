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
use std::{fs::rename, str::from_utf8, thread::sleep, time::Duration};

/// Enroll the endpoint to our server based on parsed Server.toml file
pub(crate) fn setup_enrollment(config: &mut DaemonConfig) {
    let mut enroll = match config.enroll_request() {
        Ok(result) => result,
        Err(_err) => return,
    };

    let max_attempts = 8;
    let mut count = 0;
    let pause = 6;

    // If we get `endpoint_invalid` response. We have to enroll again. We attempt 8 more enrollments max
    while enroll.endpoint_invalid && count != max_attempts {
        // Pause for 6 seconds between each attempt
        sleep(Duration::from_secs(pause));

        let enroll_attempt = match config.enroll_request() {
            Ok(result) => result,
            Err(_err) => return,
        };

        if !enroll.endpoint_invalid {
            enroll = enroll_attempt;
            break;
        }

        count += 1;
    }

    if enroll.endpoint_invalid {
        error!("[daemon] Endpoint still invalid despite 8 enrollment attempts");
        return;
    }
    config.client.daemon.endpoint_id = enroll.endpoint_id;
}

/// Process our collection request
pub(crate) fn setup_collection(config: &mut DaemonConfig, collect: &CollectResponse) {
    if collect.endpoint_invalid {
        setup_enrollment(config);
    }
    let collection_bytes = match base64_decode_standard(&collect.collection) {
        Ok(result) => result,
        Err(err) => {
            error!("[daemon] Could not decode TOML collection {err:?}");
            return;
        }
    };

    // Validate the output is JSONL and compressed
    let collect_string = String::from_utf8(collection_bytes.clone()).unwrap_or_default();
    let clean_string = collect_string.replace(" ", "");
    if !clean_string.contains("format=jsonl") && !clean_string.contains("compressed=true") {
        error!("[daemon] Invalid collection TOML. Format should be JSONL with compression");
        return;
    }

    if let Err(err) = forensics::core::parse_toml_data(&collection_bytes) {
        error!("[daemon] Could not process TOML collection {err:?}");
    }
}

/// Get a daemon configuration from our server. If none is provided we will generate a default config
pub(crate) fn setup_config(config: &mut DaemonConfig) {
    let daemon_config = match config.config_request() {
        Ok(result) => result,
        Err(_err) => return setup_daemon(config),
    };

    // Check if we got a endpoint_invalid response
    if daemon_config.endpoint_invalid {
        setup_enrollment(config);
    }

    let toml_bytes = match base64_decode_standard(&daemon_config.config) {
        Ok(result) => result,
        Err(err) => {
            error!("[daemon] Could not decode daemon config: {err:?}. Will use default config");
            return setup_daemon(config);
        }
    };

    let toml_config: DaemonToml = match toml::from_str(from_utf8(&toml_bytes).unwrap_or_default()) {
        Ok(result) => result,
        Err(err) => {
            error!("[daemon] Could not parse toml daemon config: {err:?}. Will use default config");
            return setup_daemon(config);
        }
    };

    config.client = toml_config;
    setup_daemon(config);
}

/// Move our server.toml file to our base config directory. Ex: /var/artemis/server.toml
pub(crate) fn move_server_config(path: &str, alt_artemis_path: Option<&str>) {
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

    if let Err(status) = rename(path, format!("{artemis_path}/server.toml")) {
        error!("[daemon] Could not move server.toml file to {artemis_path}: {status:?}");
    }
}

/// Setup default config directories for the daemon
fn setup_daemon(daemon_config: &mut DaemonConfig) {
    match daemon(&mut daemon_config.client, None) {
        Ok(_result) => {}
        Err(err) => {
            error!("[daemon] Could not setup daemon TOML config: {err:?}");
        }
    }
}
