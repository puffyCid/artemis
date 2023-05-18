use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub(crate) struct UnifiedLogsOptions {
    pub(crate) sources: Vec<String>,
}
