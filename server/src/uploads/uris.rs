use super::upload::upload_collection;
use crate::server::ServerState;
use axum::{routing::post, Router};

/// Setup upload routes
pub(crate) fn upload_routes(base: &str) -> Router<ServerState> {
    Router::new().route(&format!("{base}/upload"), post(upload_collection))
}

#[cfg(test)]
mod tests {
    use crate::{server::ServerState, uploads::uris::upload_routes, utils::config::read_config};
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use redb::Database;
    use std::{path::PathBuf, sync::Arc};
    use tokio::sync::broadcast;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn test_upload_routes() {
        let base = "/endpoint/v1";
        let route = upload_routes(base);

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let (clients, _rx) = broadcast::channel(100);
        let central_collect_db = Arc::new(
            Database::create("./tmp/collections14.redb")
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
                    .method(Method::POST)
                    .uri(format!("{base}/upload"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);
    }
}
