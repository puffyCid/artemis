use super::websocket::socket_connection;
use crate::server::ServerState;
use axum::{routing::get, Router};

/// Setup `Web Socket` routes
pub(crate) fn socket_routes(base: &str) -> Router<ServerState> {
    Router::new().route(&format!("{base}/socket"), get(socket_connection))
}

#[cfg(test)]
mod tests {
    use super::socket_routes;
    use crate::{server::ServerState, utils::config::read_config};
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use redb::Database;
    use std::{path::PathBuf, sync::Arc};
    use tokio::sync::broadcast;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn test_socket_routes_bad() {
        let base = "/endpoint/v1";
        let route = socket_routes(base);

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let (clients, _rx) = broadcast::channel(100);
        let central_collect_db = Arc::new(
            Database::create("./tmp/collections13.redb")
                .expect("Could not setup central collections redb"),
        );

        let server_state = ServerState {
            config,
            clients,
            central_collect_db,
        };

        let res = route
            .with_state(server_state)
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .header("connection", "upgrade")
                    .header("upgrade", "websocket")
                    .header("sec-websocket-version", "13")
                    .header("sec-websocket-key", "13")
                    .uri(format!("{base}/socket"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::UPGRADE_REQUIRED);
    }
}
