use ntfs::NtfsFileReference;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::accessor::disk::format::DiskFormat;

/// Source of our data that we want to access
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SourceId {
    /// Live OS
    #[default]
    Host,
    /// Raw NTFS filesystem
    Ntfs(char),
    /// A zip file
    Zip(PathBuf),
    /// Disk Image
    Disk {
        /// Disk image format
        format: DiskFormat,
        /// Path to the disk image
        path: PathBuf,
    },
}

impl SourceId {
    /// Return the `SourceId` as a string
    pub(crate) fn display(&self) -> String {
        match self {
            SourceId::Host => String::from("host"),
            SourceId::Ntfs(drive) => format!("ntfs:{drive}:"),
            SourceId::Zip(path) => format!("zip:{}", path.display()),
            SourceId::Disk { format, path } => format!("{}:{}", format.as_str(), path.display()),
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

/// File reference in a disk image.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DiskEntryRef {
    /// NTFS filesystem reference
    Ntfs(NtfsEntryRef),
    /// Reference to root partition
    PartitionRoot,
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
    /// Filesystem reference in a disk image
    Disk {
        /// Path to the disk image
        image: PathBuf,
        /// Disk image format
        format: DiskFormat,
        /// Partition name the file reference is on
        partition_id: String,
        /// Path to the file inside the disk image
        filesystem_path: String,
        /// Reference to file location
        entry: DiskEntryRef,
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
    /// Filesystem reference in a disk image
    Disk {
        /// Path to the disk image
        image: PathBuf,
        /// Disk image format
        format: DiskFormat,
        /// Partition name the directory reference is on
        partition_id: String,
        /// Path to the directory inside the disk image
        filesystem_path: String,
        /// Reference to directory location
        entry: DiskEntryRef,
    },
}

/// `raw:/image.raw!Partition0:hello\file.txt`, or `raw:/image.raw!Partition0` at the partition root.
pub(crate) fn disk_display_path(
    image: &Path,
    format: &DiskFormat,
    partition_id: &str,
    filesystem_path: &str,
) -> String {
    if filesystem_path.is_empty() {
        format!("{}:{}!{partition_id}", format.as_str(), image.display())
    } else {
        format!(
            "{}:{}!{partition_id}:{filesystem_path}",
            format.as_str(),
            image.display()
        )
    }
}
/// `raw:/image.raw!` for the image root, where the partitions are listed.
pub(crate) fn disk_root_display(image: &Path, format: &DiskFormat) -> String {
    format!("{}:{}!", format.as_str(), image.display())
}
