use crate::accessor::{disk::raw::RawDisk, error::AccessorResult};
use serde::{Deserialize, Serialize};
use std::{
    fs::File,
    io::{self, BufReader, Read, Seek, SeekFrom},
    path::Path,
};

/// Support disk images that the accessor can open
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum DiskFormat {
    /// Raw disk image
    Raw,
}

/// Reader for one logical disk. Each container format adds a variant.
pub(super) enum DiskReader {
    /// Reader for a `Raw` disk image
    ///
    /// We can just use a normal `BufReader` since raw disk files
    /// have no disk format
    Raw(BufReader<File>),
}

impl DiskFormat {
    /// Return a reader for the disk image
    pub(super) fn open_reader(self, path: &Path) -> AccessorResult<DiskReader> {
        match self {
            Self::Raw => Ok(DiskReader::Raw(RawDisk::new(path)?.open_reader()?)),
        }
    }

    /// Return `DiskFormat` as string
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Raw => "raw",
        }
    }
}

/// Open a logical disk reader for a disk image
///
/// Each disk image has its own reader
pub(super) trait DiskImage: Send {
    /// Reader positioned at start of the logical disk
    type Reader: Read + Seek + Send;

    /// Return a disk image reader
    fn open_reader(&self) -> AccessorResult<Self::Reader>;
}

impl Read for DiskReader {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        match self {
            Self::Raw(reader) => reader.read(buf),
        }
    }
}

impl Seek for DiskReader {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        match self {
            Self::Raw(reader) => reader.seek(pos),
        }
    }
}

/// Returns a full display path
///
/// Example: `raw:/image.raw!Partition0:hello\file.txt`, or `raw:/image.raw!Partition0` for the partition root
pub(crate) fn disk_display_path(
    image: &Path,
    format: DiskFormat,
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
/// Returns just the disk image path
///
///  `raw:/image.raw!` for the image root, where the partitions are listed.
pub(crate) fn disk_root_display(image: &Path, format: DiskFormat) -> String {
    format!("{}:{}!", format.as_str(), image.display())
}
