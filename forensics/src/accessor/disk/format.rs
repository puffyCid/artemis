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
    Raw(BufReader<File>),
}

impl DiskFormat {
    /// Return a reader for the disk image
    pub(super) fn open_reader(&self, path: &Path) -> AccessorResult<DiskReader> {
        match self {
            Self::Raw => Ok(DiskReader::Raw(RawDisk::new(path)?.open_reader()?)),
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
