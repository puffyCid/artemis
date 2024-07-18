use super::{error::SocketError, heartbeat::generate_heartbeat};
use crate::socket::collections::{quick_collection, save_collection};
use common::server::{
    collections::{CollectionRequest, QuickCollection},
    config::ArtemisConfig,
};
use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use std::time::Duration;
use tokio::time::interval;
use tokio_tungstenite::connect_async;

/// Establish websocket connection to server
pub(crate) async fn start_connection(config: &mut ArtemisConfig) -> Result<(), SocketError> {
    let version = format!("v{}", config.endpoint_server.version);
    let url = format!(
        "ws://{}:{}/endpoint/{version}/socket",
        config.endpoint_server.address, config.endpoint_server.port
    );

    let connect_result = connect_async(&url).await;
    let (socket, _response) = match connect_result {
        Ok(result) => result,
        Err(err) => {
            error!("[client] Could not connect to server: ${err:?}");
            return Err(SocketError::StartConnection);
        }
    };

    let (mut sender, mut receiver) = socket.split();
    let id = config.endpoint_id.clone();

    let beat_interval = 60;
    let mut interval = interval(Duration::from_secs(beat_interval));
    interval.tick().await;

    while let Some(message) = receiver.next().await {
        let command = match message {
            Ok(result) => {
                if !result.is_text() {
                    continue;
                }
                result.to_string()
            }
            Err(err) => {
                error!("[client] Could not understand message from websocket server: {err:?}");
                continue;
            }
        };

        info!("Received command: {command}");

        if let Ok(collection) = serde_json::from_str::<CollectionRequest>(&command) {
            if !collection.targets.contains(&config.endpoint_id) {
                continue;
            }
            save_collection(&collection, &config.endpoint_server.storage).await?;
            continue;
        }

        if let Ok(quick) = serde_json::from_str::<QuickCollection>(&command) {
            if quick.target != config.endpoint_id {
                continue;
            }
            let response_result = quick_collection(&quick).await;
            let response = match response_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[client] Could not parse quick collection: {err:?}");
                    continue;
                }
            };

            let serde_result = serde_json::to_string(&response);
            if serde_result.is_err() {
                error!(
                    "[client] Could not serialize quick response: {:?}",
                    serde_result.unwrap_err()
                );
                continue;
            }

            let status = sender.send(serde_result.unwrap().into()).await;
            if status.is_err() {
                error!(
                    "[client] Could not send quick collection: ${:?}",
                    status.unwrap_err()
                );
            }
        }
    }

    let _sent = tokio::spawn(async move {
        loop {
            interval.tick().await;

            let serde_result = serde_json::to_string(&generate_heartbeat(&id));
            if serde_result.is_err() {
                error!(
                    "[client] Could not serialize heartbeat: {:?}",
                    serde_result.unwrap_err()
                );
                continue;
            }
            let status = sender.send(serde_result.unwrap().into()).await;
            if status.is_err() {
                error!(
                    "[client] Could not send heartbeat: ${:?}",
                    status.unwrap_err()
                );
            }
        }
    });

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::filesystem::config::read_config;
    use crate::socket::connect::start_connection;
    use std::path::PathBuf;

    #[tokio::test]
    #[should_panic(expected = "StartConnection")]
    async fn test_start_connection_fail() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/client.toml");

        let mut config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        start_connection(&mut config).await.unwrap();
    }
}
