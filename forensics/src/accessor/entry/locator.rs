use crate::accessor::disk::format::DiskFormat;
use crate::accessor::entry::deserialize::deserialize_uint;
use ntfs::NtfsFileReference;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Source of our data that we want to access
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum SourceId {
    /// Live OS
    #[default]
    Host,
    /// Raw NTFS filesystem
    Ntfs(char),
    /// Raw EXT4 filesystem
    Ext4(PathBuf),
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
            SourceId::Ext4(path) => format!("ext4:{}", path.display()),
            SourceId::Zip(path) => format!("zip:{}", path.display()),
            SourceId::Disk { format, path } => format!("{}:{}", format.as_str(), path.display()),
        }
    }
}

/// Raw file reference to a file/directory on NTFS
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) struct NtfsEntryRef {
    /// NTFS File Record Number
    #[serde(deserialize_with = "deserialize_uint")]
    pub(crate) file_record_number: u64,
    /// NTFS Sequence Number
    #[serde(deserialize_with = "deserialize_uint")]
    pub(crate) sequence_number: u16,
}

/// File reference in a disk image.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub(crate) enum DiskEntryRef {
    /// NTFS filesystem reference
    Ntfs(NtfsEntryRef),
    /// EXT4 filesystem reference
    Ext4(Ext4EntryRef),
    /// Reference to root partition
    PartitionRoot,
}

/// Unique Inode and generation for a filesystem entry
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub(crate) struct Ext4EntryRef {
    /// Inode value
    #[serde(deserialize_with = "deserialize_uint")]
    pub(crate) inode: u64,
    /// Generation value for the inode
    #[serde(deserialize_with = "deserialize_uint")]
    pub(crate) generation: u32,
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// EXT4 file access requires a device label, `Ext4EntryRef`, human readable string
    Ext4 {
        /// Target of the ext4 volume. Ex: `/dev/sda2`
        label: String,
        /// Inode number and generation value
        entry: Ext4EntryRef,
        /// Human readable path
        display_path: String,
    },
    /// ZIP file access requires `PathBuf` and the entry we want access to
    Zip {
        /// Path to the zip archive
        archive: PathBuf,
        /// Index to the file in the zip archive
        #[serde(deserialize_with = "deserialize_uint")]
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
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
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
    /// EXT4 file access requires a device label, `Ext4EntryRef`, human readable string
    Ext4 {
        /// Target of the ext4 volume. Ex: `/dev/sda2`
        label: String,
        /// Inode number and generation value
        entry: Ext4EntryRef,
        /// Human readable path
        display_path: String,
    },
    Zip {
        /// Path to the zip archive
        archive: PathBuf,
        /// Index to the directory in the zip archive
        #[serde(deserialize_with = "deserialize_uint")]
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

#[cfg(test)]
mod tests {
    use super::{DirLocator, FileLocator};
    use serde_json::json;

    #[test]
    fn test_ntfs_file_ref_numbers() {
        let value = json!({
            "Ntfs": {
                "drive": "C",
                "file_ref": {"file_record_number": 3722112.0, "sequence_number": "3"},
                "display_path": "C:\\Users\\dev\\test.txt"
            }
        });

        let locator: FileLocator = serde_json::from_value(value).unwrap();
        let FileLocator::Ntfs { file_ref, .. } = locator else {
            panic!("expected Ntfs locator");
        };

        assert_eq!(file_ref.file_record_number, 3722112);
        assert_eq!(file_ref.sequence_number, 3);
    }

    #[test]
    fn test_ntfs_dir_ref_numbers() {
        let value = json!({
            "Ntfs": {
                "drive": "C",
                "dir_ref": {"file_record_number": 5000000000u64, "sequence_number": 65535},
                "display_path": "C:\\Users\\dev"
            }
        });

        let locator: DirLocator = serde_json::from_value(value).unwrap();
        let DirLocator::Ntfs { dir_ref, .. } = locator else {
            panic!("expected Ntfs locator");
        };

        assert_eq!(dir_ref.file_record_number, 5000000000);
        assert_eq!(dir_ref.sequence_number, 65535);
    }

    #[test]
    fn test_ext4_file_ref_numbers() {
        let value = json!({
            "Ext4": {
                "label": "/dev/sda2",
                "entry": {"inode": 3722112.0, "generation": 3747804916.0},
                "display_path": "/home/ubu/test.txt"
            }
        });

        let locator: FileLocator = serde_json::from_value(value).unwrap();
        let FileLocator::Ext4 { entry, .. } = locator else {
            panic!("expected Ext4 locator");
        };

        assert_eq!(entry.inode, 3722112);
        assert_eq!(entry.generation, 3747804916);
    }

    #[test]
    fn test_ext4_dir_ref_numbers() {
        let value = json!({
            "Ext4": {
                "label": "/dev/sda2",
                "entry": {"inode": "5000000000", "generation": 4294967295u32},
                "display_path": "/home/ubu"
            }
        });

        let locator: DirLocator = serde_json::from_value(value).unwrap();
        let DirLocator::Ext4 { entry, .. } = locator else {
            panic!("expected Ext4 locator");
        };

        assert_eq!(entry.inode, 5000000000);
        assert_eq!(entry.generation, u32::MAX);
    }

    #[test]
    fn test_zip_file_entry_index() {
        let value = json!({
            "Zip": {"archive": "/tmp/a.zip", "entry_index": 4.0, "entry": "a.txt"}
        });
        let locator: FileLocator = serde_json::from_value(value).unwrap();

        let FileLocator::Zip {
            entry_index, entry, ..
        } = locator
        else {
            panic!("expected Zip locator");
        };

        assert_eq!(entry_index, 4);
        assert_eq!(entry, "a.txt");
    }

    #[test]
    fn test_zip_dir_entry_index() {
        let value = json!({
            "Zip": {"archive": "/tmp/a.zip", "entry_index": "7", "prefix": "docs/"}
        });
        let locator: DirLocator = serde_json::from_value(value).unwrap();

        let DirLocator::Zip {
            entry_index,
            prefix,
            ..
        } = locator
        else {
            panic!("expected Zip locator");
        };

        assert_eq!(entry_index, 7);
        assert_eq!(prefix, "docs/");
    }

    #[test]
    fn test_zip_entry_index_rejects_overflow() {
        let value = json!({
            "Zip": {"archive": "/tmp/a.zip", "entry_index": 4294967296u64, "entry": "a.txt"}
        });
        assert!(serde_json::from_value::<FileLocator>(value).is_err());
    }
}
