use crate::{
    enrollment::enroll::enroll_client, filesystem::config::read_config,
    socket::connect::start_connection,
};
use log::error;

#[tokio::main]
pub async fn start(path: &str) {
    let config_result = read_config(path).await;
    let mut config = match config_result {
        Ok(result) => result,
        Err(err) => {
            error!("[client] Could not read config at {path}. Cannot start client without a config file: {err:?}");
            return;
        }
    };

    let enroll_status = enroll_client(&mut config).await;
    if enroll_status.is_err() {
        error!(
            "[client] Could not enroll endpoint: {:?}",
            enroll_status.unwrap_err()
        );
        return;
    }

    start_connection(&mut config)
        .await
        .expect("Websocket connection failed");
}

#[cfg(test)]
mod tests {
    use super::start;
    use std::path::PathBuf;

    #[test]
    #[ignore = "Spawns client"]
    fn test_start() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/client.toml");
        let config_path = test_location.display().to_string();
        start(&config_path)
    }
}
