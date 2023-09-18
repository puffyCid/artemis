use super::heartbeat::{parse_heartbeat, parse_pulse};
use crate::db::jobs::get_jobs;
use crate::enrollment::enroll::verify_enrollment;
use crate::server::ServerState;
use axum::extract::ws::{Message, WebSocket};
use axum::extract::{ConnectInfo, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures::{SinkExt, StreamExt};
use log::{error, warn};
use redb::Database;
use std::collections::HashMap;
use std::ops::ControlFlow::Continue;
use std::sync::Arc;
use std::{net::SocketAddr, ops::ControlFlow};

/// Accept `Web Sockets`
pub(crate) async fn socket_connection(
    State(state): State<ServerState>,
    socket: WebSocketUpgrade,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    socket.on_upgrade(move |ws| handle_socket(ws, addr, state.endpoint_db, state.job_db))
}

/// Process the `Web Socket`
async fn handle_socket(
    socket: WebSocket,
    addr: SocketAddr,
    endpoint_db: Arc<Database>,
    job_db: Arc<Database>,
) {
    let (mut sender, mut receiver) = socket.split();

    let _receive_task = tokio::spawn(async move {
        while let Some(Ok(message)) = receiver.next().await {
            let control = parse_message(&message, &addr, &endpoint_db);
            if control.is_break() {
                break;
            }

            if let Continue(id) = control {
                let jobs_result = get_jobs(&id, &job_db);
                let jobs = match jobs_result {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[server] Could not get jobs using ID {id}: {err:?}");
                        HashMap::new()
                    }
                };
                let serde_result = serde_json::to_string(&jobs);
                let serde_value = match serde_result {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[server] Could not serialize jobs for {id}: {err:?}");
                        continue;
                    }
                };

                let send_result = sender.send(Message::Text(serde_value)).await;

                if send_result.is_err() {
                    error!(
                        "[server] Could not send jobs to ID {id}: {:?}",
                        send_result.unwrap_err()
                    );
                }
            }
        }
    });
}

/// Parse `Web Socket` message
fn parse_message(message: &Message, addr: &SocketAddr, db: &Database) -> ControlFlow<(), String> {
    let ip = addr.ip().to_string();
    match message {
        Message::Text(data) => {
            if !verify_enrollment(data, &ip, db) {
                return ControlFlow::Break(());
            }
            if data.contains("\"heartbeat\"") {
                let id = parse_heartbeat(data, &ip);
                return ControlFlow::Continue(id);
            } else if data.contains("\"pulse\"") {
                let id = parse_pulse(data, &ip);
                return ControlFlow::Continue(id);
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

    ControlFlow::Continue(String::new())
}

#[cfg(test)]
mod tests {
    use super::parse_message;
    use crate::db::tables::setup_db;
    use crate::socket::websocket::Message::Text;
    use std::net::{IpAddr, Ipv4Addr, SocketAddr};
    use std::ops::ControlFlow::Continue;
    use std::path::PathBuf;

    #[test]
    fn test_parse_message() {
        let message = Text(String::from(
            r#"{"endpoint_id":"3482136c-3176-4272-9bd7-b79f025307d6","hostname":"hello","platform":"Darwin","boot_time":0,"os_version":"12.0","uptime":110,"kernel_version":"12.1","heartbeat":true,"timestamp":1111111,"jobs_running":0,"cpu":[{"frequency":0,"cpu_usage":25.70003890991211,"name":"1","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":25.076454162597656,"name":"2","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":8.922499656677246,"name":"3","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":6.125399112701416,"name":"4","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":4.081260681152344,"name":"5","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":3.075578451156616,"name":"6","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":2.0113024711608887,"name":"7","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":1.5097296237945557,"name":"8","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":1.288386583328247,"name":"9","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10},{"frequency":0,"cpu_usage":1.1674108505249023,"name":"10","vendor_id":"Apple","brand":"Apple M1 Max","physical_core_count":10}],"disks":[{"disk_type":"SSD","file_system":"97112102115","mount_point":"/","total_space":494384795648 ,"available_space":295755320592 ,"removable":false},{"disk_type":"SSD","file_system":"97112102115","mount_point":"/System/Volumes/Data","total_space":494384795648 ,"available_space":295755320592 ,"removable":false}],"memory":{"available_memory":20146110464 ,"free_memory":6238076928 ,"free_swap":0,"total_memory":34359738368 ,"total_swap":0,"used_memory":18717523968 ,"used_swap":0}}"#,
        ));
        let address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8000);

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/endpoints.redb");
        let path = test_location.display().to_string();
        let db = setup_db(&path).unwrap();

        let control = parse_message(&message, &address, &db);
        if let Continue(value) = control {
            assert_eq!(value, "3482136c-3176-4272-9bd7-b79f025307d6");
        }
    }
}
