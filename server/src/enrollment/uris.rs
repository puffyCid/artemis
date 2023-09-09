use super::enroll::enroll_endpoint;
use crate::server::ServerState;
use axum::{routing::post, Router};

/// Setup `Enrollment` routes
pub(crate) fn enroll_routes(base: &str) -> Router<ServerState> {
    Router::new().route(&format!("{base}/enroll"), post(enroll_endpoint))
}

#[cfg(test)]
mod tests {
    use super::enroll_routes;
    use crate::{db::tables::setup_db, server::ServerState, utils::config::read_config};
    use axum::{
        body::Body,
        http::{Method, Request, StatusCode},
    };
    use std::path::PathBuf;
    use tower::util::ServiceExt;

    #[tokio::test]
    async fn test_enroll_routes() {
        let base = "/endpoint/v1";
        let route = enroll_routes(base);
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");

        let config = read_config(&test_location.display().to_string()).unwrap();
        let endpointdb = setup_db(&format!(
            "{}/endpoints.redb",
            &config.endpoint_server.storage
        ))
        .unwrap();

        let jobdb = setup_db(&format!("{}/jobs.redb", &config.endpoint_server.storage)).unwrap();

        let state_server = ServerState {
            config,
            endpoint_db: endpointdb,
            job_db: jobdb,
        };
        let res = route
            .with_state(state_server)
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri(format!("{base}/enroll"))
                    .header("content-type", "application/json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(res.status(), StatusCode::BAD_REQUEST);

        let body = hyper::body::to_bytes(res.into_body()).await.unwrap();
        assert_eq!(body, "Failed to parse the request body as JSON: EOF while parsing a value at line 1 column 0")
    }
}
