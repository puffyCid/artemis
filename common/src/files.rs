use serde::Serialize;

#[cfg(target_os = "macos")]
use crate::macos::MachoInfo;
#[cfg(target_os = "windows")]
use crate::windows::PeInfo;

#[derive(Debug, Serialize)]
pub struct FileInfo {
    pub full_path: String,
    pub directory: String,
    pub filename: String,
    pub extension: String,
    pub created: i64,
    pub modified: i64,
    pub changed: i64,
    pub accessed: i64,
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
    #[cfg(target_os = "macos")]
    pub binary_info: Vec<MachoInfo>,
    #[cfg(target_os = "windows")]
    pub binary_info: Vec<PeInfo>,
    #[cfg(target_os = "linux")]
    pub binary_info: Vec<ElfInfo>,
}
