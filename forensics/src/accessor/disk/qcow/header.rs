use nom::number::complete::{be_u8, be_u32, be_u64};
use tracing::warn;

use crate::{
    accessor::error::{AccessorError, AccessorResult},
    utils::nom_helper::{nom_bu8, nom_bu32, nom_bu64, nom_take},
};

/// Header info for QCOW file
///
/// Version 2 and 3 use the layout documented at
/// <https://github.com/qemu/qemu/blob/master/docs/interop/qcow2.rst>
#[derive(Debug, PartialEq, Clone)]
pub(super) struct QcowHeader {
    /// QCOW version
    version: QcowVersion,
    /// Backing file name location
    ///
    /// None if there no backing file
    backing_file: Option<BackingFile>,
    /// Bits per cluster
    ///
    /// Size is `cluster_size` = 1 << `cluster_bits`
    cluster_bits: u32,
    /// Virtual disk size in bytes
    virtual_size: u64,
    /// Encryption method used by the QCOW disk
    encryption: Encryption,
    /// Number of entries in the active Level 1 table
    l1_entries: u32,
    /// File offset of the active Level 1 table
    l1_offset: u64,
    /// File offset of the refcount table
    refcount_offset: u64,
    /// Number of clusters occupied by the refcount table
    refcount_clusters: u32,
    /// Number of snapshots
    snapshot_count: u32,
    /// File offset to the snapshot table
    snapshot_offset: u64,
    /// Incompatible flag bitmask
    incompatible: u64,
    /// Compatible flag bitmask
    compatible: u64,
    /// Autoclear flag bitmask
    autoclear: u64,
    /// Recount table order
    ///
    /// Refcount bits is `refcount_bits` = 1 << `refcount_order`
    refcount_order: u32,
    /// Header length in bytes
    header_length: u32,
    /// Compression used by the QCOW disk
    compression: Compression,
}

/// Location of a backing file associated with the disk
///
/// Backing file is not parsed when parsing the header
#[derive(PartialEq, Debug, Clone, Copy)]
struct BackingFile {
    /// Offset to the backing file name in disk
    offset: u64,
    /// Length of the name in bytes
    ///
    /// Not null terminated
    size: u32,
}

/// QCOW disk version
#[derive(PartialEq, Debug, Clone, Copy)]
enum QcowVersion {
    /// Version 2. 72 bytes
    V2,
    /// Version 3. At least 104 bytes
    V3,
}

/// Encryption used by the QCOW disk
#[derive(PartialEq, Debug, Clone, Copy)]
enum Encryption {
    /// Data is not encrypted
    None,
    /// Data is encrypted with AES
    Aes,
    /// Data is encrypted with LUKS
    Luks,
}

/// Compression used by the QCOW disk
#[derive(PartialEq, Debug, Clone, Copy)]
enum Compression {
    /// Data is compressed with zlib
    ///
    /// Also referred to as `Deflate`: <https://www.qemu.org/docs/master/interop/qcow2.html#additional-fields-version-3-and-higher> and
    /// <https://gitlab.com/qemu-project/qemu/-/blob/585f8587ad3619cf070fdc19d2d0b8e7a2398d91/block-qcow2.c#L1182>
    Zlib,
    /// Data is compressed with Zstandard
    Zstd,
}

impl QcowHeader {
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

    /// Incompatible header flags
    const INCOMPATIBLE_FLAGS: u64 =
        Self::DIRTY | Self::CORRUPT | Self::EXTERNAL_DATA | Self::COMPRESSION | Self::EXTENDED_L2;

    /// Return whether the disk is marked as dirty
    fn is_dirty(&self) -> bool {
        self.incompatible & Self::DIRTY != 0
    }

    /// Return whether the disk is marked as corrupt
    fn is_corrupt(&self) -> bool {
        self.incompatible & Self::CORRUPT != 0
    }

    /// Return whether the disk has external data file
    fn has_external_data(&self) -> bool {
        self.incompatible & Self::EXTERNAL_DATA != 0
    }

    /// Return whether the disk L2 entries uses extended 128-bit layout
    fn has_extended_l2(&self) -> bool {
        self.incompatible & Self::EXTENDED_L2 != 0
    }

    /// Return whether the disk uses compression
    fn has_compression_type(&self) -> bool {
        self.incompatible & Self::COMPRESSION != 0
    }

    fn unknown_incompatible(&self) -> u64 {
        self.incompatible & !Self::INCOMPATIBLE_FLAGS
    }

    pub(super) fn check_supported(&self) -> AccessorResult<()> {
        let unknown = self.unknown_incompatible();
        if unknown != 0 {
            return Err(AccessorError::volume(format!(
                "QCOW image has unknown incompatible feature bits {unknown:#x}"
            )));
        }

        if self.has_external_data() {
            return Err(AccessorError::volume(format!(
                "QCOW external data files are not supported"
            )));
        }

        if self.has_extended_l2() {
            return Err(AccessorError::volume(
                "QCOW extended L2 entries are not supported",
            ));
        }

        if self.encryption != Encryption::None {
            return Err(AccessorError::volume(format!(
                "QCOW encryption {:?} is not supported",
                self.encryption
            )));
        }

        if let Some(backing) = self.backing_file {
            return Err(AccessorError::volume(format!(
                "QCOW backing file at offset {} is not supported",
                backing.offset
            )));
        }

        Ok(())
    }
}

pub(super) fn parse_header(data: &[u8]) -> AccessorResult<QcowHeader> {
    let (input, sig) = nom_take(data, 4_u8, "QCOW header sig is truncated")?;
    let magic = b"QFI\xfb";

    if sig != magic {
        return Err(AccessorError::volume(format!(
            "Invalid QCOW magic {sig:?}. Wanted QFI\\xfb"
        )));
    }

    let (input, version_raw) = nom_bu32(input, "QCOW version is truncated")?;
    let version = match version_raw {
        2 => QcowVersion::V2,
        3 => QcowVersion::V3,
        _ => {
            return Err(AccessorError::volume(format!(
                "Unsupported QCOW version {version_raw}"
            )));
        }
    };

    let (input, backing_offset) = nom_bu64(input, "QCOW backing file offset is truncated")?;
    let (input, backing_size) = nom_bu32(input, "QCOW backing file size is truncated")?;
    let (input, cluster_bits) = nom_bu32(input, "QCOW cluster bits are truncated")?;
    let (input, virtual_size) = nom_bu64(input, "QCOW virtual size is truncated")?;
    let (input, encryption_raw) = nom_bu32(input, "QCOW encryption method is truncated")?;

    let (input, l1_entries) = nom_bu32(input, "QCOW L1 entry count is truncated")?;
    let (input, l1_offset) = nom_bu64(input, "QCOW L1 offset is truncated")?;
    let (input, refcount_offset) = nom_bu64(input, "QCOW refcount table offset is truncated")?;
    let (input, refcount_clusters) =
        nom_bu32(input, "QCOW refcount table cluster count is truncated")?;
    let (input, snapshot_count) = nom_bu32(input, "QCOW snapshot count is truncated")?;
    let (remaining, snapshot_offset) = nom_bu64(input, "QCOW snapshot offset is truncated")?;

    let fixed = FixedFields {
        version,
        backing_file: backing_file(backing_offset, backing_size)?,
        cluster_bits,
        virtual_size,
        encryption: encryption(encryption_raw)?,
        l1_entries,
        l1_offset,
        refcount_offset,
        refcount_clusters,
        snapshot_count,
        snapshot_offset,
    };

    match version {
        QcowVersion::V2 => Ok(fixed.into_version2()),
        QcowVersion::V3 => parse_version3(remaining, data.len(), fixed),
    }
}

/// Fields shared by QCOW version 2 and version 3
struct FixedFields {
    /// QCOW version
    version: QcowVersion,
    /// Backing file name location
    ///
    /// None if there no backing file
    backing_file: Option<BackingFile>,
    /// Bits per cluster
    ///
    /// Size is `cluster_size` = 1 << `cluster_bits`
    cluster_bits: u32,
    /// Virtual disk size in bytes
    virtual_size: u64,
    /// Encryption method used by the QCOW disk
    encryption: Encryption,
    /// Number of entries in the active Level 1 table
    l1_entries: u32,
    /// File offset of the active Level 1 table
    l1_offset: u64,
    /// File offset of the refcount table
    refcount_offset: u64,
    /// Number of clusters occupied by the refcount table
    refcount_clusters: u32,
    /// Number of snapshots
    snapshot_count: u32,
    /// File offset to the snapshot table
    snapshot_offset: u64,
}

impl FixedFields {
    /// Return a version 2 `QcowHeader`
    fn into_version2(self) -> QcowHeader {
        QcowHeader {
            version: self.version,
            backing_file: self.backing_file,
            cluster_bits: self.cluster_bits,
            virtual_size: self.virtual_size,
            encryption: self.encryption,
            l1_entries: self.l1_entries,
            l1_offset: self.l1_offset,
            refcount_offset: self.refcount_offset,
            refcount_clusters: self.refcount_clusters,
            snapshot_count: self.snapshot_count,
            snapshot_offset: self.snapshot_offset,
            incompatible: 0,
            compatible: 0,
            autoclear: 0,
            refcount_order: 4,
            header_length: 72,
            compression: Compression::Zlib,
        }
    }
}

/// Parse the additional fields found in version 3 of the QCOW disk
fn parse_version3(
    input: &[u8],
    data_length: usize,
    fixed: FixedFields,
) -> AccessorResult<QcowHeader> {
    let (input, incompatible) = nom_bu64(input, "QCOW incompatible features are truncated")?;
    let (input, compatible) = nom_bu64(input, "QCOW compatible features are truncated")?;
    let (input, autoclear) = nom_bu64(input, "QCOW autoclear features are truncated")?;
    let (input, refcount_order) = nom_bu32(input, "QCOW refcount order is truncated")?;
    let (input, header_length) = nom_bu32(input, "QCOW header length is truncated")?;

    let min_length = 104;
    if header_length < header_length || header_length % 8 != 0 {
        return Err(AccessorError::volume(format!(
            "Invalid QCOW header length {header_length}. Version 3 headers are at least {min_length} bytes and a multiple of 8"
        )));
    }

    if data_length < header_length as usize {
        return Err(AccessorError::volume(format!(
            "QCOW header is {data_length} bytes. Need {header_length}"
        )));
    }

    let max_refcount_order = 6;
    if refcount_order > max_refcount_order {
        return Err(AccessorError::volume(format!(
            "Invalid QCOW refcount order {refcount_order}. Maximum order is {max_refcount_order}"
        )));
    }

    Ok(QcowHeader {
        version: fixed.version,
        backing_file: fixed.backing_file,
        cluster_bits: fixed.cluster_bits,
        virtual_size: fixed.virtual_size,
        encryption: fixed.encryption,
        l1_entries: fixed.l1_entries,
        l1_offset: fixed.l1_offset,
        refcount_offset: fixed.refcount_offset,
        refcount_clusters: fixed.refcount_clusters,
        snapshot_count: fixed.snapshot_count,
        snapshot_offset: fixed.snapshot_offset,
        incompatible,
        compatible,
        autoclear,
        refcount_order,
        header_length,
        compression: compression(input, header_length, incompatible)?,
    })
}

/// Check for valid backing file
fn backing_file(offset: u64, size: u32) -> AccessorResult<Option<BackingFile>> {
    if offset == 0 {
        return Ok(None);
    }

    let max_name_size = 1023;
    if size == 0 || size > max_name_size {
        return Err(AccessorError::volume(format!(
            "QCOW backing file name is {size} bytes. Valid lengths are 1 through {max_name_size}"
        )));
    }

    Ok(Some(BackingFile { offset, size }))
}

/// Determine encryption algorithm
fn encryption(value: u32) -> AccessorResult<Encryption> {
    match value {
        0 => Ok(Encryption::None),
        1 => Ok(Encryption::Aes),
        2 => Ok(Encryption::Luks),
        _ => Err(AccessorError::volume(format!(
            "Unknown QCOW encryption method {value}"
        ))),
    }
}

/// Determine compression algorithm
fn compression(input: &[u8], header_length: u32, incompatible: u64) -> AccessorResult<Compression> {
    let type_bit = incompatible & QcowHeader::COMPRESSION != 0;

    let compression_offset = 104;
    if header_length <= compression_offset {
        if type_bit {
            return Err(AccessorError::volume(
                "QCOW compression type bit is set, but the compression field is absent",
            ));
        }

        return Ok(Compression::Zlib);
    }

    let (_remaining, value) = nom_bu8(input, "QCOW compression type is truncated")?;

    match (type_bit, value) {
        (false, 0) => Ok(Compression::Zlib),
        (true, 1) => Ok(Compression::Zstd),
        (true, 0) => Err(AccessorError::volume(
            "QCOW compression type bit is set, but the compression type is deflate",
        )),
        (false, found) => Err(AccessorError::volume(format!(
            "QCOW compression type is {found}, but the compression type bit is clear"
        ))),
        (true, found) => Err(AccessorError::volume(format!(
            "Unsupported QCOW compression type {found}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use crate::accessor::disk::qcow::header::{Compression, QcowVersion, parse_header};
    use std::{fs::read, path::PathBuf};

    #[test]
    fn test_parse_header() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/disks/qcow/headers/header_version3.raw");
        let bytes = read(test_location.to_str().unwrap()).unwrap();

        let result = parse_header(&bytes).unwrap();

        assert_eq!(result.virtual_size, 85899345920);
        assert_eq!(result.cluster_bits, 16);
        assert_eq!(result.l1_entries, 160);
        assert_eq!(result.l1_offset, 262144);
        assert_eq!(result.version, QcowVersion::V3);
        assert_eq!(result.compression, Compression::Zlib);
    }

    #[test]
    fn test_get_header() {
        let test = [
            81, 70, 73, 251, 0, 0, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0,
            20, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 160, 0, 0, 0, 0, 0, 4, 0, 0, 0, 0, 0, 0, 0, 1, 0,
            0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 4, 0, 0, 0, 112, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let result = parse_header(&test).unwrap();

        assert!(result.backing_file.is_none());
        assert_eq!(result.snapshot_offset, 0);
        assert_eq!(result.snapshot_count, 0);
    }
}
