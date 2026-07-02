use ntfs::NtfsFileReference;
use std::path::PathBuf;

/// Source of our data that we want to access
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) enum SourceId {
    /// Live OS
    Host,
    /// Raw NTFS filesystem
    RawNtfs(char),
    /// A zip file
    Zip(PathBuf),
}

impl SourceId {
    /// Return the `SourceId` as a string
    pub(crate) fn display(&self) -> String {
        match self {
            SourceId::Host => String::from("host"),
            SourceId::RawNtfs(drive) => format!("raw:{drive}"),
            SourceId::Zip(path) => format!("zip:{}", path.display()),
        }
    }
}

/// Raw file reference to a file/directory on NTFS
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct NtfsEntryRef {
    /// NTFS File Record Number
    pub(crate) file_record_number: u64,
    /// NTFS Sequence Number
    pub(crate) sequence_number: u16,
}

impl NtfsEntryRef {
    /// Create a `NtfsEntryRef` from `NtfsFileReference`
    pub(crate) fn from_reference(reference: NtfsFileReference) -> Self {
        Self {
            file_record_number: reference.file_record_number(),
            sequence_number: reference.sequence_number(),
        }
    }
}

/// Requirements to locate a file from a provided source
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FileLocator {
    /// We just need a `PathBuf` to access a file on live OS
    Host {
        /// Path to the file
        path: PathBuf,
    },
    /// NTFS file access requires drive leter, `NtfsEntryRef`, human readable string
    Ntfs {
        /// Drive letter
        drive: char,
        /// NTFS file reference we to the file
        file_ref: NtfsEntryRef,
        /// Human readable path
        display_path: String,
    },
    /// ZIP file access requires `PathBuf` and the entry we want access to
    Zip {
        /// Path to the zip archive
        archive: PathBuf,
        /// Index to the file in the zip archive
        entry_index: u32,
        /// Path to the file in the zip
        entry: String,
    },
}

/// Requirements to locate a directory from a provided source
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DirLocator {
    /// We just need a `PathBuf` to access a directory on live OS
    Host {
        /// Path to the directory
        path: PathBuf,
    },
    Ntfs {
        /// Drive letter
        drive: char,
        /// NTFS file reference we to the file
        dir_ref: NtfsEntryRef,
        /// Human readable path
        display_path: String,
    },
    Zip {
        /// Path to the zip archive
        archive: PathBuf,
        /// Index to the directory in the zip archive
        entry_index: u32,
        /// Path to the directory in the zip
        prefix: String,
    },
}
