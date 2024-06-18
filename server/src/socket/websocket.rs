use super::command::parse_command;
use super::heartbeat::parse_heartbeat;
use super::jobs::parse_job;
use crate::enrollment::enroll::verify_enrollment;
use crate::filestore::jobs::get_jobs;
use crate::server::ServerState;
use crate::socket::command::quick_jobs;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use log::{error, warn};
use std::collections::HashMap;
use std::ops::ControlFlow::Continue;
use std::{net::SocketAddr, ops::ControlFlow};
use tokio::sync::mpsc;

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

    let mut message_source = MessageSource::None;
    let mut id = String::new();
    /*
     * When a system first connects over websockets we need to determine source of message:
     * 1. MessageSource::Client - Socket connection is remote system
     * 2. MessageSource::Server - Socket connection local server
     *
     *  MessageSource::Client will return the endpoint_id as the `socket_message`
     *  MessageSource::Server will return the entire message as the `socket_message`
     */
    while let Some(Ok(message)) = receiver.next().await {
        let control = parse_message(&message, &addr, &storage_path).await;
        if control.is_break() {
            break;
        }

        if let Continue(socket_message) = control {
            message_source = socket_message.source;
            id = socket_message.id;
            break;
        }
    }

    // If the message is None, don't setup any async tasks
    if message_source == MessageSource::None {
        warn!("[server] Unexpected message source none");
        return;
    }

    /*
     * Register new endpoint clients and use ID to track channels to send websocket commands from server
     * When client system first checks in setup an async task and channel.
     * This async task will be used to communicate with single client. This task is stored in a shared state (HashMap) to track individual clients
     */
    if message_source == MessageSource::Client && state.command.read().await.get(&id).is_none() {
        let (client_send, mut client_recv) = mpsc::channel(50);

        let _send_task = tokio::spawn(async move {
            while let Some(msg) = client_recv.recv().await {
                // If any websocket error, break loop.
                let result = sender.send(msg).await;
                if result.is_err() {
                    error!(
                        "[server] Could not send server message: {:?}",
                        result.unwrap_err()
                    );
                    break;
                }
            }
        });

        // Register sender associated with endpoint client. Tracked via Endpoint ID
        state.command.write().await.insert(id.clone(), client_send);
    }

    /*
     * After we have registerd an async task for a client. Spawn another task to receive websocket data.
     * Types of websocket data:
     *  - Server commands
     *  - Client heartbeat
     *  - Client jobs
     */
    let _recv_task = tokio::spawn(async move {
        while let Some(Ok(message)) = receiver.next().await {
            // Parse the websocket data
            let control = parse_message(&message, &addr, &storage_path).await;
            // If the client disconnects from us, we need to remove from our tracker. We can no longer send commands from server
            if control.is_break() {
                state.command.write().await.remove(&id);
                break;
            }

            if let Continue(socket_message) = control {
                if socket_message.source == MessageSource::None {
                    continue;
                }

                // If the source is the Server then the socket_data contains a command to be sent the client
                if socket_message.source == MessageSource::Server {
                    let send_result =
                        quick_jobs(&socket_message.content, &state.command.read().await).await;
                    if send_result.is_err() {
                        error!(
                            "[server] Could not issue quick job command: {:?}",
                            send_result.unwrap_err()
                        );
                    }
                    continue;
                }

                /*
                 * The message source should now be Client. socket_data = endpoint_id
                 * The function `parse_message` already handles the heartbeat
                 *
                 * At this point we just need the endpoint_id to check for collection jobs
                 */
                let endpoint_path = format!(
                    "{storage_path}/{}/{}",
                    socket_message.platform, socket_message.id
                );
                let jobs_result = get_jobs(&endpoint_path).await;
                let jobs = match jobs_result {
                    Ok(result) => result,
                    Err(err) => {
                        error!(
                            "[server] Could not get jobs using ID {}: {err:?}",
                            socket_message.id
                        );
                        HashMap::new()
                    }
                };

                // Serialize the available collection jobs if any
                let serde_result = serde_json::to_string(&jobs);
                let serde_value = match serde_result {
                    Ok(result) => result,
                    Err(err) => {
                        error!(
                            "[server] Could not serialize jobs for {}: {err:?}",
                            socket_message.id
                        );
                        continue;
                    }
                };

                // Get the registered socket sender associated with the registered async task for the Client.
                // This was registered when the client first checked into the server
                if let Some(client_send) = state.command.read().await.get(&socket_message.id) {
                    let send_result = client_send.send(Message::Text(serde_value)).await;
                    if send_result.is_err() {
                        error!(
                            "[server] Could not send jobs to ID {}: {:?}",
                            socket_message.id,
                            send_result.unwrap_err()
                        );
                    }
                }
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
            if data.contains("\"targets\":") && ip == "127.0.0.1" {
                let command = parse_command(data, path).await;
                if command.is_err() {
                    error!(
                        "[server] Could not parse the server command: {:?}",
                        command.unwrap_err()
                    );
                    return ControlFlow::Break(());
                }
                socket_message.source = MessageSource::Server;
                socket_message.content = data.to_string();

                // Send the command the to targets
                return ControlFlow::Continue(socket_message);
            }
            if data.contains("\"job\":") {
                let job = parse_job(data, &ip, path).await;
                if job.is_err() {
                    error!(
                        "[server] Could not parse the job result: {:?}",
                        job.unwrap_err()
                    );
                    return ControlFlow::Break(());
                }
                return ControlFlow::Continue(socket_message);
            }

            if verify_enrollment(data, &ip, path).is_err() {
                return ControlFlow::Break(());
            }

            socket_message.source = MessageSource::Client;
            if data.contains("\"heartbeat\":") {
                let (id, plat) = parse_heartbeat(data, &ip, path).await;
                socket_message.id = id;
                socket_message.platform = plat;
                return ControlFlow::Continue(socket_message);
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
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::ops::ControlFlow::Continue;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_parse_message() {
        let message = Text(String::from(
            r#"{"endpoint_id":"3482136c-3176-4272-9bd7-b79f025307d6","hostname":"hello","platform":"Darwin","boot_time":0,"os_version":"12.0","uptime":110,"kernel_version":"12.1","heartbeat":true,"timestamp":1111111,"jobs_running":0,"cpu":[{"frequency":0,"cpu_usage":25.70003890991211,"name":"1","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":25.076454162597656,"name":"2","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":8.922499656677246,"name":"3","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":6.125399112701416,"name":"4","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":4.081260681152344,"name":"5","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":3.075578451156616,"name":"6","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":2.0113024711608887,"name":"7","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":1.5097296237945557,"name":"8","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":1.288386583328247,"name":"9","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":1.1674108505249023,"name":"10","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10}],"disks":[{"disk_type":"SSD","file_system":"97112102115","mount_point":"/","total_space":494384795648 ,"available_space":295755320592 ,"removable":false},{"disk_type":"SSD","file_system":"97112102115","mount_point":"/System/Volumes/Data","total_space":494384795648 ,"available_space":295755320592 ,"removable":false}],"memory":{"available_memory":20146110464 ,"free_memory":6238076928 ,"free_swap":0,"total_memory":34359738368 ,"total_swap":0,"used_memory":18717523968 ,"used_swap":0}}"#,
        ));
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data");
        let path = test_location.display().to_string();

        let control = parse_message(&message, &address, &path).await;
        if let Continue(socket_message) = control {
            assert_eq!(socket_message.id, "3482136c-3176-4272-9bd7-b79f025307d6");
            assert_eq!(socket_message.source, MessageSource::Client)
        }
    }
}
