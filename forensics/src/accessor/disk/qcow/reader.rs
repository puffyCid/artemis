use crate::accessor::disk::qcow::level::{Level2Cache, QcowMap};

/// Reader for the guest OS inside a QCOW disk image
#[derive(Debug)]
pub(super) struct QcowReader<R> {
    /// Reader for the disk file
    image: R,
    /// Offset of the guest OS
    position: u64,
    /// Level 1 table and cluster size
    qcow: QcowMap,
    /// Cached level 2 tables
    cache: Level2Cache,
}
