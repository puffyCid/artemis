use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
use crate::linux::ElfInfo;
#[cfg(target_os = "macos")]
use crate::macos::MachoInfo;
#[cfg(target_os = "windows")]
use crate::windows::PeInfo;

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
    #[cfg(target_os = "macos")]
    pub binary_info: Vec<MachoInfo>,
    #[cfg(target_os = "windows")]
    pub binary_info: Vec<PeInfo>,
    #[cfg(target_os = "linux")]
    pub binary_info: Vec<ElfInfo>,
}
