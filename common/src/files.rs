use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize)]
/// Supported hashes
pub struct Hashes {
    pub md5: bool,
    pub sha1: bool,
    pub sha256: bool,
}
#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub full_path: String,
    pub directory: String,
    pub filename: String,
    pub extension: String,
    pub created: String,
    pub modified: String,
    pub changed: String,
    pub accessed: String,
    pub size: u64,
    pub inode: u64,
    pub mode: u32,
    pub uid: u32,
    pub gid: u32,
    pub md5: String,
    pub sha1: String,
    pub sha256: String,
    pub is_file: bool,
    pub is_directory: bool,
    pub is_symlink: bool,
    pub depth: usize,
    pub yara_hits: Vec<String>,
    pub binary_info: Value,
}
