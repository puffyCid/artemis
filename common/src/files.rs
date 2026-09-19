use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::windows::{ADSInfo, CompressionType, Namespace};

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
    pub inode: Option<u64>,
    pub attributes: Option<Vec<Attributes>>,
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

/// File entry attributes
///
/// Windows: <https://learn.microsoft.com/en-us/windows/win32/fileio/file-attribute-constants>
/// Unix: Read, Write, Execute
#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum Attributes {
    /// Windows
    ReadOnly,
    /// Windows
    Hidden,
    /// Windows
    System,
    /// Windows
    Directory,
    /// Windows
    Archive,
    /// Windows
    Device,
    /// Windows
    Normal,
    /// Windows
    Temporary,
    /// Windows
    Sparse,
    /// Windows
    ReparsePoint,
    /// Windows
    Compressed,
    /// Windows
    Offline,
    /// Windows
    NotContentIndexed,
    /// Windows
    Encrypted,
    /// Windows
    IntegritySystem,
    /// Windows
    Virtual,
    /// Windows
    NoScrubData,
    /// Windows
    ExtendedAttributes,
    /// Windows
    Pinned,
    /// Windows
    Unpinned,
    /// Windows
    RecallOnOpen,
    /// Windows
    RecallOnDataAccess,
    /// Unix
    UserRead,
    /// Unix
    GroupRead,
    /// Unix
    OtherRead,
    /// Unix
    UserWrite,
    /// Unix
    GroupWrite,
    /// Unix
    OtherWrite,
    /// Unix
    UserExecute,
    /// Unix
    GroupExecute,
    /// Unix
    OtherExecute,
    /// Unix
    SetUid,
    /// Unix
    SetGid,
    /// Unix
    Sticky,
}

#[derive(Debug, Serialize, Default)]
pub struct FileNtfsInfo {
    pub full_path: String,
    pub directory: String,
    pub filename: String,
    pub extension: String,
    pub created: String,
    pub modified: String,
    pub changed: String,
    pub accessed: String,
    pub filename_created: String,
    pub filename_modified: String,
    pub filename_changed: String,
    pub filename_accessed: String,
    pub attributes: Vec<Attributes>,
    pub size: u64,
    pub md5: String,
    pub sha1: String,
    pub sha256: String,
    pub kind: EntryKind,
    pub depth: usize,
    pub yara_hits: Vec<String>,
    pub binary_info: Value,
    pub display_path: String,
    pub compressed_size: u64,
    pub compression_type: CompressionType,
    pub inode: u64,
    pub sequence_number: u16,
    pub parent_sequence_number: u16,
    pub parent_mft_reference: u32,
    pub owner: u32,
    pub namespace: Namespace,
    pub ads_info: Vec<ADSInfo>,
    pub usn: u64,
    pub sid: u32,
    pub user_sid: String,
    pub group_sid: String,
    pub drive: String,
    pub evidence: String,
}
