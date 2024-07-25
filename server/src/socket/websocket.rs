use super::heartbeat::parse_heartbeat;
use crate::enrollment::enroll::verify_enrollment;
use crate::filestore::cache::processes::save_processes;
use crate::filestore::collections::{get_endpoint_collections_notstarted, set_collection_status};
use crate::filestore::database::save_collection;
use crate::server::ServerState;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use common::server::collections::{
    CollectionRequest, CollectionType, QuickCollection, QuickResponse, Status,
};
use common::server::heartbeat::Heartbeat;
use futures::{SinkExt, StreamExt};
use log::{error, warn};
use redb::Database;
use std::ops::ControlFlow::Continue;
use std::{net::SocketAddr, ops::ControlFlow};

/// Accept websockets
pub(crate) async fn socket_connection(
    socket: WebSocketUpgrade,
    State(state): State<ServerState>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    socket.on_upgrade(move |ws| handle_socket(ws, addr, state))
}

/// Parse all websocket communications
async fn handle_socket(socket: WebSocket, addr: SocketAddr, state: ServerState) {
    let (mut sender, mut receiver) = socket.split();
    let storage_path = state.config.endpoint_server.storage.clone();

    let mut rx = state.clients.subscribe();

    let _send_message = tokio::spawn(async move {
        while let Ok(message) = rx.recv().await {
            let send_result = sender.send(Message::Text(message)).await;
            if send_result.is_err() {
                error!(
                    "[server] Could not send message: {:?}",
                    send_result.unwrap_err()
                );
            }
            //}
        }
    });

    /*
     * After we have registerd an async task for a client. Spawn another task to receive websocket data.
     */
    let _recv_task = tokio::spawn(async move {
        while let Some(Ok(message)) = receiver.next().await {
            // Parse the websocket data
            let control = parse_message(
                &message,
                &addr,
                &state.config.endpoint_server.storage,
                &state.central_collect_db,
                &storage_path,
            )
            .await;
            if control.is_break() {
                break;
            }

            if let Continue(socket_message) = control {
                if socket_message.source == MessageSource::None {
                    continue;
                }

                // If the source is the Server then the socket_data contains a command to be sent the client
                if socket_message.source == MessageSource::Server {
                    let send_result = state.clients.send(socket_message.content);
                    if send_result.is_err() {
                        error!(
                            "[server] Could not send server command to client {}: {:?}",
                            socket_message.id,
                            send_result.unwrap_err()
                        );
                    }
                    continue;
                }

                /*
                 * The message source should now be Client. socket_data = endpoint_id
                 */
                let endpoint_path = format!(
                    "{}/{}/{}",
                    storage_path, socket_message.platform, socket_message.id
                );

                let collects_result = get_endpoint_collections_notstarted(&endpoint_path).await;
                let collects = match collects_result {
                    Ok(result) => result,
                    Err(err) => {
                        error!(
                            "[server] Could not get collections using ID {}: {err:?}",
                            socket_message.id
                        );
                        continue;
                    }
                };

                if collects.is_empty() {
                    continue;
                }

                // Serialize the available collections if any
                let serde_result = serde_json::to_string(&collects);
                let serde_value = match serde_result {
                    Ok(result) => result,
                    Err(err) => {
                        error!(
                            "[server] Could not serialize collections for {}: {err:?}",
                            socket_message.id
                        );
                        continue;
                    }
                };

                let send_result = state.clients.send(serde_value);
                if send_result.is_err() {
                    error!(
                        "[server] Could not send collections to receiver {}: {:?}",
                        socket_message.id,
                        send_result.unwrap_err()
                    );
                }
                let mut ids = Vec::new();
                for entry in collects {
                    ids.push(entry.id);
                }

                let _status = set_collection_status(&endpoint_path, &ids, &Status::Started).await;
            }
        }
    });
}

#[derive(PartialEq, Debug)]
enum MessageSource {
    Client,
    Server,
    None,
}

#[derive(PartialEq, Debug)]
struct SocketMessage {
    id: String,
    platform: String,
    source: MessageSource,
    content: String,
}

/// Parse websocket message. Currently messages are either Server messages (commands) or client messages (heartbeat)
async fn parse_message(
    message: &Message,
    addr: &SocketAddr,
    path: &str,
    central_db: &Database,
    storage_path: &str,
) -> ControlFlow<(), SocketMessage> {
    let ip = addr.ip().to_string();
    let mut socket_message = SocketMessage {
        id: String::new(),
        platform: String::new(),
        source: MessageSource::None,
        content: String::new(),
    };
    match message {
        Message::Text(data) => {
            if let Ok(_quick) = serde_json::from_str::<QuickCollection>(data) {
                if ip != "127.0.0.1" {
                    error!("[server] Received quick collection request from non-server IP");
                    return ControlFlow::Break(());
                }
                socket_message.source = MessageSource::Server;
                socket_message.content = data.to_string();

                // Send the command the to targets
                return ControlFlow::Continue(socket_message);
            }

            if let Ok(collection) = serde_json::from_str::<CollectionRequest>(data) {
                if ip != "127.0.0.1" {
                    error!("[server] Received collection request from non-server IP");
                    return ControlFlow::Break(());
                }
                socket_message.source = MessageSource::Server;
                socket_message.content = data.to_string();

                let _ = save_collection(collection, central_db, storage_path).await;

                // Send the command the to targets
                return ControlFlow::Continue(socket_message);
            }

            socket_message.source = MessageSource::Client;

            if let Ok(beat) = serde_json::from_str::<Heartbeat>(data) {
                if verify_enrollment(&beat.endpoint_id, &beat.platform, path).is_err() {
                    return ControlFlow::Break(());
                }

                let (id, plat) = parse_heartbeat(data, &ip, path).await;
                socket_message.id = id;
                socket_message.platform = plat;

                return ControlFlow::Continue(socket_message);
            }

            if let Ok(quick_response) = serde_json::from_str::<QuickResponse>(data) {
                if verify_enrollment(&quick_response.id, &quick_response.platform, path).is_err() {
                    return ControlFlow::Break(());
                }

                let endpoint_dir =
                    format!("{path}/{}/{}", quick_response.platform, quick_response.id);
                match quick_response.collection_type {
                    CollectionType::Processes => {
                        save_processes(&quick_response.data, &endpoint_dir).await;
                    }
                    CollectionType::Filelist => {}
                }
            }
        }
        Message::Binary(_) => {
            warn!("[server] Binary data unexpected");
        }
        Message::Close(_data) => {
            return ControlFlow::Break(());
        }
        Message::Ping(_data) | Message::Pong(_data) => {}
    }

    // For unsupported messages return None
    ControlFlow::Continue(socket_message)
}

#[cfg(test)]
mod tests {
    use super::parse_message;
    use crate::socket::websocket::Message::Text;
    use crate::socket::websocket::MessageSource;
    use redb::Database;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::ops::ControlFlow::Continue;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_parse_message() {
        let message = Text(String::from(
            r#"{"endpoint_id":"3482136c-3176-4272-9bd7-b79f025307d6","timestamp":22,"jobs_running":100,"boot_time":1693527587,"hostname":"aStudio.lan","os_version":"13.2","uptime":4550,"kernel_version":"22.3.0","platform":"Darwin","ip":"127.0.0.1","artemis_version":"0.9.0","cpu":[{"frequency":0,"cpu_usage":25.70003890991211,"name":"1","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":25.076454162597656,"name":"2","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":8.922499656677246,"name":"3","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":6.125399112701416,"name":"4","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":4.081260681152344,"name":"5","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":3.075578451156616,"name":"6","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":2.0113024711608887,"name":"7","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":1.5097296237945557,"name":"8","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":1.288386583328247,"name":"9","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":1.1674108505249023,"name":"10","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10}],"disks":[{"disk_type":"SSD","file_system":"97112102115","mount_point":"/","total_space":494384795648,"available_space":295755320592,"removable":false},{"disk_type":"SSD","file_system":"97112102115","mount_point":"/System/Volumes/Data","total_space":494384795648,"available_space":295755320592,"removable":false}],"memory":{"available_memory":20146110464,"free_memory":6238076928,"free_swap":0,"total_memory":34359738368,"total_swap":0,"used_memory":18717523968,"used_swap":0}}"#,
        ));
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data");
        let path = test_location.display().to_string();

        let db = Database::create("./tmp/test.redb").unwrap();

        let control = parse_message(&message, &address, &path, &db, "./tmp").await;
        if let Continue(socket_message) = control {
            assert_eq!(socket_message.id, "3482136c-3176-4272-9bd7-b79f025307d6");
            assert_eq!(socket_message.source, MessageSource::Client)
        }
    }
}
