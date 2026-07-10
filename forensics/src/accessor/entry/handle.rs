use crate::accessor::entry::locator::{DirLocator, FileLocator};
use std::path::PathBuf;

/// Support data entries we can access
///
/// Right now we only support reading files or directories
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum EntryKind {
    /// Entry is a file
    File,
    /// Entry is a directory
    Directory,
    /// Entry is unsupported
    Unsupported,
}

/// Metadata returned from stat, glob, and directory listing.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EntryMeta {
    /// `EntryKind` type
    pub(crate) kind: EntryKind,
    /// Size of entry
    pub(crate) size: u64,
    /// Human readable path to the entry
    pub(crate) display_path: String,
}

impl EntryMeta {
    /// Create a `EntryMeta` value
    pub(crate) fn new(kind: EntryKind, size: u64, display_path: impl Into<String>) -> Self {
        Self {
            kind,
            size,
            display_path: display_path.into(),
        }
    }
}

/// Handle to a file
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FileHandle {
    /// Location to the file entry
    pub(crate) locator: FileLocator,
}

impl FileHandle {
    /// Create a `FileHandle` value
    pub(crate) fn new(locator: FileLocator) -> Self {
        Self { locator }
    }

    /// Return a `FileHandle` on a live OS
    pub(crate) fn host(path: impl Into<PathBuf>) -> Self {
        Self::new(FileLocator::Host { path: path.into() })
    }

    /// Return a `FileHandle` as a string
    pub(crate) fn display_path(&self) -> String {
        match &self.locator {
            FileLocator::Host { path } => path.display().to_string(),
            FileLocator::Ntfs { display_path, .. } => display_path.clone(),
            FileLocator::Zip { archive, entry, .. } => {
                format!("zip:{}!{entry}", archive.display())
            }
        }
    }
}

/// Handle to a directory
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DirHandle {
    /// Location to the directory entry
    pub(crate) locator: DirLocator,
}

impl DirHandle {
    /// Create a `DirHandle` value
    pub(crate) fn new(locator: DirLocator) -> Self {
        Self { locator }
    }

    /// Return a `DirHandle` on a live OS
    pub(crate) fn host(path: impl Into<PathBuf>) -> Self {
        Self::new(DirLocator::Host { path: path.into() })
    }

    /// Return a `DirHandle` as a string
    pub(crate) fn display_path(&self) -> String {
        match &self.locator {
            DirLocator::Host { path } => path.display().to_string(),
            DirLocator::Ntfs { display_path, .. } => display_path.clone(),
            DirLocator::Zip {
                archive, prefix, ..
            } => {
                if prefix.is_empty() {
                    format!("zip:{}", archive.display())
                } else {
                    format!("zip:{}!{prefix}", archive.display())
                }
            }
        }
    }
}

/// Result of a glob operation. The handle can be passed directly to read APIs
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct GlobMatch {
    /// Glob match to a file
    pub(crate) handle: ItemHandle,
    /// Metadata associated with our file match
    pub(crate) meta: EntryMeta,
}

impl GlobMatch {
    /// Create a `GlobMatch` value
    pub(crate) fn new(handle: ItemHandle, meta: EntryMeta) -> Self {
        Self { handle, meta }
    }
}

/// Handle returned for one child of a directory listing
///
/// Files and symlinks use `FileHandle`. Subdirectories use `DirHandle` so callers
/// can call `list_dir` again without re-walking from the volume root
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ItemHandle {
    File(FileHandle),
    Directory(DirHandle),
    Unsupported(FileHandle),
}

impl ItemHandle {
    /// Return the `EntryKind` for the `ItemHandle`
    pub(crate) fn kind(&self) -> EntryKind {
        match self {
            Self::File(_) => EntryKind::File,
            Self::Directory(_) => EntryKind::Directory,
            Self::Unsupported(_) => EntryKind::Unsupported,
        }
    }

    /// Return the path for the `ItemHandle`
    pub(crate) fn display_path(&self) -> String {
        match self {
            Self::File(handle) | Self::Unsupported(handle) => handle.display_path(),
            Self::Directory(handle) => handle.display_path(),
        }
    }

    /// Return the `FileHandle` for the `ItemHandle`
    pub(crate) fn as_file(&self) -> Option<&FileHandle> {
        match self {
            Self::File(handle) => Some(handle),
            Self::Directory(_) | Self::Unsupported(_) => None,
        }
    }

    /// Return the `DirHandle` for the `ItemHandle`
    pub(crate) fn as_directory(&self) -> Option<&DirHandle> {
        match self {
            Self::Directory(handle) => Some(handle),
            Self::File(_) | Self::Unsupported(_) => None,
        }
    }
}

/// One row from a directory listing
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DirEntry {
    /// Name of entry
    pub(crate) name: String,
    /// Handle to the file
    pub(crate) handle: ItemHandle,
    /// Metadata associated with our entry
    pub(crate) meta: EntryMeta,
}
impl DirEntry {
    /// Create a `DirEntry` value
    pub(crate) fn new(name: impl Into<String>, handle: ItemHandle, meta: EntryMeta) -> Self {
        Self {
            name: name.into(),
            handle,
            meta,
        }
    }

    /// Determine if `ItemHandle` is a directory
    pub(crate) fn is_directory(&self) -> bool {
        matches!(self.handle, ItemHandle::Directory(_))
    }

    /// Determine if `ItemHandle` is a file
    pub(crate) fn is_file(&self) -> bool {
        matches!(self.handle, ItemHandle::File(_))
    }
}
