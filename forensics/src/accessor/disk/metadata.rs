use crate::accessor::disk::{
    format::DiskFormat,
    qcow::header::{BackingFile, Compression, Encryption, QcowVersion},
};

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

/// Metadata for a QCOW image
///
/// Most data comes from the header
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct QcowImageMetadata {
    /// QCOW version
    pub(crate) version: QcowVersion,
    /// Bits per cluster
    pub(crate) cluster_bits: u32,
    /// Size of the QCOW cluster
    pub(crate) cluster_size: u64,
    /// Number of entries in the active Level 1 table
    pub(crate) l1_entries: u32,
    /// File offset of the active Level 1 table
    pub(crate) l1_offset: u64,
    /// File offset of the refcount table
    pub(crate) refcount_offset: u64,
    /// Number of clusters occupied by the refcount table
    pub(crate) refcount_clusters: u32,
    /// Refcount table order
    refcount_order: u32,
    pub(crate) snapshot_count: u32,
    /// File offset to the snapshot table
    pub(crate) snapshot_offset: u64,
    /// Encryption method used by the QCOW disk
    pub(crate) encryption: Encryption,
    /// Backing file name location. <https://www.bitosis.nl/posts/qemu-backing-files>
    ///
    /// None if there no backing file
    pub(crate) backing_file: Option<BackingFile>,
    /// Compression used by the QCOW disk
    pub(crate) compression: Compression,
    pub(crate) flags: QcowFeatureFlags,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct QcowFeatureFlags {
    /// Incompatible feature bitmask.
    pub(crate) incompatible: u64,
    /// Compatible feature bitmask.
    pub(crate) compatible: u64,
    /// Autoclear feature bitmask.
    pub(crate) autoclear: u64,
    /// Incompatible bit 0.
    pub(crate) dirty: bool,
    /// Incompatible bit 1.
    pub(crate) corrupt: bool,
    /// Incompatible bit 2.
    pub(crate) external_data: bool,
    /// Incompatible bit 3.
    pub(crate) compression_type: bool,
    /// Incompatible bit 4.
    pub(crate) extended_l2: bool,
    /// Compatible bit 0.
    pub(crate) lazy_refcounts: bool,
    /// Autoclear bit 0.
    pub(crate) bitmaps: bool,
    /// Autoclear bit 1.
    pub(crate) raw_external_data: bool,
}

/// QCOW header feature flags
impl QcowFeatureFlags {
    /// Has `DIRTY` flag
    const DIRTY: u64 = 1 << 0;
    /// Has `CORRUPT` flag
    const CORRUPT: u64 = 1 << 1;
    /// Has `EXTERNAL_DATA` flag
    const EXTERNAL_DATA: u64 = 1 << 2;
    /// Has `COMPRESSION` flag
    const COMPRESSION: u64 = 1 << 3;
    /// Has `EXTENDED_L2` flag
    const EXTENDED_L2: u64 = 1 << 4;
    /// has `LAZY_REFCOUNTS` flag
    const LAZY_REFCOUNTS: u64 = 1 << 0;
    /// Has `BITMAPS` flag
    const BITMAPS: u64 = 1 << 0;
    /// Has `RAW_EXTERNAL_DATA` flag
    const RAW_EXTERNAL_DATA: u64 = 1 << 1;

    /// Determine the feature bits and preserve the original masks.
    pub(crate) fn from_masks(incompatible: u64, compatible: u64, autoclear: u64) -> Self {
        Self {
            incompatible,
            compatible,
            autoclear,
            dirty: incompatible & Self::DIRTY != 0,
            corrupt: incompatible & Self::CORRUPT != 0,
            external_data: incompatible & Self::EXTERNAL_DATA != 0,
            compression_type: incompatible & Self::COMPRESSION != 0,
            extended_l2: incompatible & Self::EXTENDED_L2 != 0,
            lazy_refcounts: compatible & Self::LAZY_REFCOUNTS != 0,
            bitmaps: autoclear & Self::BITMAPS != 0,
            raw_external_data: autoclear & Self::RAW_EXTERNAL_DATA != 0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::QcowFeatureFlags;

    #[test]
    fn test_feature_flags_name_known_bits_and_keep_unknown_bits() {
        let incompatible = (1 << 0) | (1 << 4) | (1 << 10);
        let compatible = 1 << 0;
        let autoclear = (1 << 0) | (1 << 1);
        let flags = QcowFeatureFlags::from_masks(incompatible, compatible, autoclear);

        assert_eq!(flags.incompatible, incompatible);
        assert_eq!(flags.compatible, compatible);
        assert_eq!(flags.autoclear, autoclear);

        assert!(flags.dirty);
        assert!(!flags.corrupt);
        assert!(!flags.external_data);

        assert!(!flags.compression_type);
        assert!(flags.extended_l2);

        assert!(flags.lazy_refcounts);
        assert!(flags.bitmaps);
        assert!(flags.raw_external_data);
    }
}
