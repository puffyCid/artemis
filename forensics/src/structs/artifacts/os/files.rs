use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct FileOptions {
    pub start_path: String,
    pub depth: Option<u32>,
    pub metadata: bool,
    pub md5: bool,
    pub sha1: bool,
    pub sha256: bool,
    pub path_regex: Option<String>,
    pub filename_regex: Option<String>,
    pub yara: Option<String>,
    pub exclude_directories: Option<Vec<String>>,
    pub source: String,
    pub verbose: bool,
}
