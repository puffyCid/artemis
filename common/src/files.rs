use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Deserialize)]
/// Supported hashes
pub struct Hashes {
    pub md5: bool,
    pub sha1: bool,
    pub sha256: bool,
}
#[derive(Debug, Serialize, Default)]
pub struct FileInfo {
    pub full_path: String,
    pub directory: String,
    pub filename: String,
    pub extension: String,
    pub created: Option<String>,
    pub modified: Option<String>,
    pub changed: Option<String>,
    pub accessed: Option<String>,
    pub filename_created: Option<String>,
    pub filename_modified: Option<String>,
    pub filename_changed: Option<String>,
    pub filename_accessed: Option<String>,
    pub uid: Option<String>,
    pub gid: Option<String>,
    pub inode: Option<i64>,
    pub size: u64,
    pub md5: String,
    pub sha1: String,
    pub sha256: String,
    pub kind: EntryKind,
    pub depth: usize,
    pub yara_hits: Vec<String>,
    pub binary_info: Value,
    pub display_path: String,
}

/// Support data entries we can access
///
/// Right now we only support reading files or directories
#[derive(Debug, Clone, PartialEq, Serialize, Default)]
pub enum EntryKind {
    /// Entry is a file
    File,
    /// Entry is a directory
    Directory,
    /// Entry is a symbolic link
    Symlink,
    /// Entry is a socket
    Socket,
    /// Entry is a block device
    BlockDevice,
    /// Entry is named pipe
    Pipe,
    /// Entry is character device
    CharDevice,
    /// Entry is unsupported
    #[default]
    Unsupported,
}
