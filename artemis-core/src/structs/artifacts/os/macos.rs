use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct UnifiedLogsOptions {
    pub sources: Vec<String>,
}
