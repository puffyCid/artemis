use crate::utils::info::server_info;
use axum::Json;
use common::server::webui::ServerInfo;

/// Poll basic information about server resources
pub(crate) async fn about() -> Json<ServerInfo> {
    let info = server_info();

    Json(info)
}

#[cfg(test)]
mod tests {
    use crate::frontend::about::about;

    #[tokio::test]
    async fn test_about() {
        let result = about().await;
        assert!(result.total_memory > 0)
    }
}
