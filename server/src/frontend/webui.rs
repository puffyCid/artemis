use axum::http::{header, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use log::warn;
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "../target/dist/web"]
struct Frontend;

/// Serve the compiled WebAssembly (WASM) binary and CSS to the client
pub(crate) async fn webui() -> Result<Response, StatusCode> {
    match Frontend::get("index.html") {
        Some(result) => Ok(Html(result.data).into_response()),
        None => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

/// Return the remaining web assets the the WASM binary
pub(crate) async fn webui_assets(uri: Uri) -> Result<Response, StatusCode> {
    let path = uri.path().trim_start_matches('/');
    match Frontend::get(path) {
        Some(result) => {
            let content_type = if path.ends_with("wasm") {
                "application/wasm"
            } else if path.ends_with("js") {
                "application/javascript"
            } else if path.ends_with("css") {
                "text/css"
            } else {
                warn!("[server] Unsupported content request: {path}");
                return Err(StatusCode::UNSUPPORTED_MEDIA_TYPE);
            };

            Ok(([(header::CONTENT_TYPE, content_type)], result.data).into_response())
        }
        None => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

#[cfg(test)]
mod tests {
    use crate::frontend::webui::{webui, webui_assets};
    use axum::http::{StatusCode, Uri};

    #[tokio::test]
    async fn test_webui() {
        let result = webui().await.unwrap();
        assert_eq!(result.status(), StatusCode::OK)
    }

    #[tokio::test]
    #[should_panic(expected = "500")]
    async fn test_webui_assets() {
        let _ = webui_assets(Uri::from_static("http://127.0.0.1/badrequest"))
            .await
            .unwrap();
    }
}
