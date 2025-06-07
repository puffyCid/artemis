use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ProcessOptions {
    pub md5: bool,
    pub sha1: bool,
    pub sha256: bool,
    pub metadata: bool,
}
