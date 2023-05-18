use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct FileOptions {
    pub start_path: String,
    pub depth: Option<u8>,
    pub metadata: Option<bool>,
    pub md5: Option<bool>,
    pub sha1: Option<bool>,
    pub sha256: Option<bool>,
    pub regex_filter: Option<String>,
}
