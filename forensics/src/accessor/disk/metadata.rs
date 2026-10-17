use crate::accessor::disk::{format::DiskFormat, qcow::reader::QcowImageMetadata};

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct DiskImageMetadata {
    /// Disk image format
    pub(crate) format: DiskFormat,
    /// Size of the disk image
    pub(crate) file_size: u64,
    /// Size of the guest OS
    pub(crate) virtual_size: u64,
    /// Metadata associated with the disk image
    pub(crate) details: DiskImageDetails,
}

/// Metadata associated with each disk type
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DiskImageDetails {
    /// Raw disks have very minimal metadata
    Raw,
    /// QCOW metadata
    Qcow(QcowImageMetadata),
}
