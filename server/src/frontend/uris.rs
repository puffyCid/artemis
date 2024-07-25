use super::{
    about::about,
    collections::{endpoint_quick, get_collections_db},
    endpoints::{endpoint_info, endpoint_list, endpoint_processes, endpoint_stats},
    webui::webui,
};
use crate::server::ServerState;
use axum::{
    routing::{get, post},
    Router,
};

/// Setup `WebUI` routes
pub(crate) fn setup_webui(base: &str) -> Router<ServerState> {
    // Setup pages
    let mut frontend = Router::new().route(&format!("{base}/home"), get(webui));
    frontend = frontend.merge(Router::new().route(&format!("{base}/about"), get(webui)));
    frontend = frontend.merge(Router::new().route(&format!("{base}/endpoints"), get(webui)));
    frontend = frontend.merge(Router::new().route(&format!("{base}/endpoints/info"), get(webui)));
    frontend = frontend.merge(Router::new().route(&format!("{base}/collections"), get(webui)));

    // Post requests for Endpoint info
    frontend = frontend
        .merge(Router::new().route(&format!("{base}/endpoint/stats"), post(endpoint_stats)));
    frontend =
        frontend.merge(Router::new().route(&format!("{base}/endpoint/list"), post(endpoint_list)));
    frontend =
        frontend.merge(Router::new().route(&format!("{base}/endpoints/info"), post(endpoint_info)));
    frontend = frontend.merge(Router::new().route(
        &format!("{base}/endpoints/processes"),
        post(endpoint_processes),
    ));

    // Requests for collections
    frontend = frontend.merge(Router::new().route(
        &format!("{base}/collections/list"),
        post(get_collections_db),
    ));

    // Post requests for collections
    frontend = frontend
        .merge(Router::new().route(&format!("{base}/endpoints/quick"), post(endpoint_quick)));

    // Server stats
    frontend = frontend.merge(Router::new().route(&format!("{base}/server/stats"), get(about)));
    frontend
}

#[cfg(test)]
mod tests {
    use crate::{frontend::uris::setup_webui, server::ServerState, utils::config::read_config};
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use redb::Database;
    use std::{path::PathBuf, sync::Arc};
    use tokio::sync::broadcast;
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

        let (clients, _rx) = broadcast::channel(100);
        let central_collect_db = Arc::new(
            Database::create("./tmp/collections12.redb")
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
                    .uri(format!("{base}/home"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::OK);
    }
}
