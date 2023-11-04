use crate::{
    enrollment::uris::enroll_routes,
    frontend::{uris::setup_webui, webui::webui_assets},
    server::ServerState,
    socket::uris::socket_routes,
    uploads::uris::upload_routes,
};
use axum::{routing::get, Router};

/// Setup all the server routes
pub(crate) fn setup_routes() -> Router<ServerState> {
    let mut app = Router::new();

    app = app.route("/", get(|| async { "Hello, World!" }));

    let version = "v1";
    let endpoint_base = format!("/endpoint/{version}");

    app = app.merge(enroll_routes(&endpoint_base));
    app = app.merge(socket_routes(&endpoint_base));
    app = app.merge(upload_routes(&endpoint_base));

    let webui_base = format!("/ui/{version}");
    app = app.merge(setup_webui(&webui_base));

    app = app.fallback(webui_assets);
    app
}

#[cfg(test)]
mod tests {
    use super::setup_routes;
    use crate::{server::ServerState, utils::config::read_config};
    use axum::{
        body::Body,
        http::{Request, StatusCode},
    };
    use std::{collections::HashMap, path::PathBuf, sync::Arc};
    use tokio::sync::RwLock;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_setup_routes() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();

        let command = Arc::new(RwLock::new(HashMap::new()));
        let server_state = ServerState { config, command };

        let app = setup_routes();
        let res = app
            .with_state(server_state)
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);

        let body = hyper::body::to_bytes(res.into_body()).await.unwrap();
        assert_eq!(&body[..], b"Hello, World!");
    }
}
