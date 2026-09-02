use crate::accessor::{
    entry::locator::{DirLocator, FileLocator},
    io::reader::{extension_from_filename, filename_from_display},
    location::scheme::{Scheme, strip_scheme},
};
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

/// Metadata returned from glob and directory listing.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct EntryMeta {
    /// `EntryKind` type
    pub(crate) kind: EntryKind,
    /// Size of entry
    pub(crate) size: u64,
    /// Human readable path to the entry
    pub(crate) full_path: String,
    /// Filename of for the entry
    pub(crate) filename: String,
    /// Extension for the filename if any
    pub(crate) extension: String,
    /// Human readable path to the entry with `Scheme`
    pub(crate) display_path: String,
}

impl EntryMeta {
    /// Create a `EntryMeta` value
    pub(crate) fn new(kind: EntryKind, size: u64, display_path: impl Into<String>) -> Self {
        let path = display_path.into();
        let filename = filename_from_display(&path);
        Self {
            kind,
            size,
            full_path: strip_scheme(&path).to_string(),
            extension: extension_from_filename(&filename),
            filename,
            display_path: path,
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
    ///
    /// Use `display_path` if you want the current `Scheme` in the path prefix
    pub(crate) fn full_path(&self) -> String {
        match &self.locator {
            FileLocator::Host { path } => path.display().to_string(),
            FileLocator::Ntfs { display_path, .. } => display_path.clone(),
            FileLocator::Zip { archive, entry, .. } => {
                format!("{}!{entry}", archive.display())
            }
        }
    }

    /// Return the filename from a `FileHandle` as a string
    ///
    /// Example: `zip:test.zip!./test.txt` returns `test.txt`. NTFS ADS will return the ADS name if provided
    pub(crate) fn filename(&self) -> String {
        filename_from_display(&self.display_path())
    }

    /// Return the extension from a `FileHandle` as a string
    ///
    /// Example: `zip:test.zip!./test.txt` returns `txt`
    pub(crate) fn extension(&self) -> String {
        extension_from_filename(&self.filename())
    }

    /// Return a `FileHandle` as a string
    ///
    /// Example: `zip:test.zip!./test.txt` or `ntfs:C:\\Users\\dev\\test.txt`
    pub(crate) fn display_path(&self) -> String {
        match &self.locator {
            FileLocator::Host { path } => format!("host:{}", path.display()),
            FileLocator::Ntfs { display_path, .. } => format!("ntfs:{}", display_path),
            FileLocator::Zip { archive, entry, .. } => {
                format!("zip:{}!{entry}", archive.display())
            }
        }
    }

    /// Return the `Scheme` associated with the `FileHandle`
    pub(crate) fn scheme(&self) -> Scheme {
        match &self.locator {
            FileLocator::Host { .. } => Scheme::Host,
            FileLocator::Ntfs { .. } => Scheme::Ntfs,
            FileLocator::Zip { .. } => Scheme::Zip,
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
            DirLocator::Host { path } => format!("host:{}", path.display()),
            DirLocator::Ntfs { display_path, .. } => format!("ntfs:{}", display_path),
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

    /// Return a `DirHandle` as a string
    ///
    /// Use `display_path` if you want the current `Scheme` in the path prefix
    pub(crate) fn full_path(&self) -> String {
        match &self.locator {
            DirLocator::Host { path } => path.display().to_string(),
            DirLocator::Ntfs { display_path, .. } => display_path.clone(),
            DirLocator::Zip {
                archive, prefix, ..
            } => {
                if prefix.is_empty() {
                    archive.display().to_string()
                } else {
                    format!("{}!{prefix}", archive.display())
                }
            }
        }
    }

    /// Return the filename from a `DirHandle` as a string
    ///
    /// Example: `zip:test.zip!./test` returns `test`
    pub(crate) fn filename(&self) -> String {
        filename_from_display(&self.display_path())
    }

    /// Return the extension from a `DirHandle` as a string
    ///
    /// Example: `zip:test.zip!./test` returns empty string
    pub(crate) fn extension(&self) -> String {
        extension_from_filename(&self.filename())
    }

    /// Return the `Scheme` associated with the `DirHandle`
    pub(crate) fn scheme(&self) -> Scheme {
        match &self.locator {
            DirLocator::Host { .. } => Scheme::Host,
            DirLocator::Ntfs { .. } => Scheme::Ntfs,
            DirLocator::Zip { .. } => Scheme::Zip,
        }
    }
}

/// Result of a glob operation
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
/// Files use `FileHandle`. Directories use `DirHandle`
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ItemHandle {
    /// A file handle to read data
    File(FileHandle),
    /// A directory handle to list additional files or directories
    Directory(DirHandle),
    /// Unsupported handle
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
    ///
    /// Includes the `Scheme` prefix
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

/// Directory value from a directory listing
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

#[derive(Debug)]
pub(crate) struct EntryStat {
    pub(crate) meta: EntryMeta,
    pub(crate) times: Vec<Timestamp>,
}

#[derive(Debug)]
pub(crate) enum Timestamp {
    Created(String),
    Modified(String),
    Accessed(String),
    Changed(String),
    FilenameCreated(String),
    FilenameModified(String),
    FilenameAccessed(String),
    FilenameChanged(String),
}
