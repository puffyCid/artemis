use crate::accessor::error::AccessorResult;
use serde::{Deserialize, Serialize};
use std::io::{Read, Seek};

/// Support disk images that the accessor can open
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub(crate) enum DiskFormat {
    /// Raw disk image
    Raw,
}

/// Open a logical disk reader for an disk image
///
/// Each disk image has its own reader
pub(super) trait DiskImage: Send {
    /// Reader positioned at start of the logical disk
    type Reader: Read + Seek + Send;

    /// Return a disk image reader
    fn open_reader(&self) -> AccessorResult<Self::Reader>;
}
