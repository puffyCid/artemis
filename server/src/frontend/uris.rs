use super::{endpoints::endpoint_stats, webui::webui};
use crate::server::ServerState;
use axum::{
    routing::{get, post},
    Router,
};

/// Setup `WebUI` routes
pub(crate) fn setup_webui(base: &str) -> Router<ServerState> {
    let mut frontend = Router::new().route(&format!("{base}/home"), get(webui));
    frontend = frontend
        .merge(Router::new().route(&format!("{base}/endpoint_stats"), post(endpoint_stats)));
    frontend
}

#[cfg(test)]
mod tests {
    use crate::{frontend::uris::setup_webui, server::ServerState, utils::config::read_config};
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use std::{collections::HashMap, path::PathBuf, sync::Arc};
    use tokio::sync::RwLock;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_setup_webui() {
        let base = "/ui/v1";
        let route = setup_webui(base);

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let command = Arc::new(RwLock::new(HashMap::new()));
        let server_state = ServerState { config, command };

        let res = route
            .with_state(server_state)
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri(format!("{base}/home"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }
}
