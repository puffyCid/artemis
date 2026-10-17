use crate::{
    accessor::{
        disk::qcow::{
            decompress::decompress_cluster,
            header::{BackingFile, Compression, Encryption, QcowHeader, QcowVersion, parse_header},
            level::{GuestMapping, Level2Cache, QcowMap},
        },
        error::{AccessorError, AccessorResult},
    },
    utils::nom_helper::nom_bu32,
};
use serde::Serialize;
use std::{
    cmp::min,
    io::{self, ErrorKind, Read, Seek, SeekFrom},
};
use tracing::warn;

/// Reader for the guest OS inside a QCOW disk image
#[derive(Debug)]
pub(crate) struct QcowReader<R> {
    /// Reader for the disk file
    image: R,
    /// Offset of the guest OS
    position: u64,
    /// Level 1 table and cluster size
    qcow: QcowMap,
    /// Cached level 2 tables
    cache: Level2Cache,
    /// Decompressed guest bytes
    decompress: Option<DecompressCluster>,
}

/// Decompressed bytes associated with guest OS
#[derive(Debug)]
struct DecompressCluster {
    /// Guest cluster index for the decompressed bytes
    guest_index: u64,
    /// Decompressed bytes
    bytes: Vec<u8>,
}

impl<R: Read + Seek> QcowReader<R> {
    /// Return a `QcowReader` for a disk image
    ///
    /// The reader can be used to read the guest OS data
    pub(super) fn open(mut image: R) -> AccessorResult<Self> {
        let file_len = image_length(&mut image)?;
        let header = read_header(&mut image, file_len)?;

        header.check_supported()?;

        if header.is_dirty() {
            warn!("opening a QCOW image marked dirty");
        }

        if header.is_corrupt() {
            warn!("opening a QCOW image marked corrupt");
        }

        let qcow = QcowMap::load(&mut image, header, file_len)?;

        Ok(Self {
            image,
            position: 0,
            qcow,
            cache: Level2Cache::new(),
            decompress: None,
        })
    }

    /// Virtual disk size in bytes
    pub(super) fn virtual_size(&self) -> u64 {
        self.qcow.header.virtual_size
    }

    /// Return the current position of the `QcowReader`
    pub(super) fn position(&self) -> u64 {
        self.position
    }

    /// Size of the QCOW file in bytes
    pub(super) fn file_size(&self) -> u64 {
        self.qcow.file_len
    }

    /// Return QCOW metadata
    pub(super) fn image_metadata(&self) -> QcowImageMetadata {
        let header = &self.qcow.header;

        QcowImageMetadata {
            version: header.version,
            cluster_bits: header.cluster_bits,
            cluster_size: self.qcow.cluster.cluster_size,
            l1_entries: header.l1_entries,
            l1_offset: header.l1_offset,
            refcount_offset: header.refcount_offset,
            refcount_clusters: header.refcount_clusters,
            refcount_order: header.refcount_order,
            snapshot_count: header.snapshot_count,
            snapshot_offset: header.snapshot_offset,
            encryption: header.encryption,
            backing_file: header.backing_file,
            compression: header.compression,
            flags: QcowFeatureFlags::from_masks(
                header.incompatible,
                header.compatible,
                header.autoclear,
            ),
        }
    }

    /// Read guest OS bytes in the QCOW disk image
    fn read_guest(&mut self, buf: &mut [u8]) -> AccessorResult<usize> {
        let virtual_size = self.virtual_size();
        if buf.is_empty() || self.position >= virtual_size {
            return Ok(0);
        }

        let remaining = virtual_size - self.position;
        let required = match usize::try_from(remaining) {
            Ok(result) => min(result, buf.len()),
            Err(_) => buf.len(),
        };

        let mut filled = 0;
        while filled < required {
            match self.read_cluster(&mut buf[filled..required]) {
                Ok(result) => filled += result,
                Err(_) if filled > 0 => return Ok(filled),
                Err(err) => return Err(err),
            }
        }

        Ok(filled)
    }

    /// Move the guest reader in the QCOW disk image
    fn seek_guest(&mut self, pos: SeekFrom) -> AccessorResult<u64> {
        let next = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::Current(delta) => checked_seek(self.position, delta)?,
            SeekFrom::End(delta) => checked_seek(self.virtual_size(), delta)?,
        };

        self.position = next;

        Ok(self.position)
    }

    /// Read guest OS bytes from the QCOW disk image cluster
    fn read_cluster(&mut self, buf: &mut [u8]) -> AccessorResult<usize> {
        let cluster_size = self.qcow.cluster.cluster_size;
        let offset_in_cluster = self.qcow.cluster.offset_in_cluster(self.position);

        let cluster_left = usize::try_from(cluster_size - offset_in_cluster).map_err(|err| {
            AccessorError::volume(format!("QCOW cluster does not fit in memory: {err:?}"))
        })?;

        let bytes = min(buf.len(), cluster_left);
        if bytes == 0 {
            return Err(AccessorError::volume(
                "QCOW read returned no bytes inside the virtual disk",
            ));
        }

        let mapping = self
            .qcow
            .map_guest(&mut self.image, &mut self.cache, self.position)
            .map_err(|err| AccessorError::volume(format!("QCOW guest data invalid: {err:}")))?;

        match mapping {
            GuestMapping::Unallocated | GuestMapping::Zero => buf[..bytes].fill(0),
            GuestMapping::Data { host_offset } => {
                let host = host_offset.checked_add(offset_in_cluster).ok_or_else(|| {
                    AccessorError::volume(format!("QCOW host offset {host_offset} overflow"))
                })?;

                self.image.seek(SeekFrom::Start(host))?;
                if let Err(err) = self.image.read_exact(&mut buf[..bytes]) {
                    if err.kind() == ErrorKind::UnexpectedEof {
                        return Err(AccessorError::Volume {
                            reason: format!(
                                "QCOW host read of {bytes} bytes at offset {host} extends past the image"
                            ),
                        });
                    }

                    return Err(AccessorError::Io {
                        path: None,
                        source: err,
                    });
                }
            }
            GuestMapping::Compressed {
                host_offset,
                compressed_size,
            } => {
                self.copy_decompressed(
                    &mut buf[..bytes],
                    host_offset,
                    compressed_size,
                    offset_in_cluster,
                )?;
            }
        }

        self.position = self
            .position
            .checked_add(u64::try_from(bytes).map_err(|err| AccessorError::Volume {
                reason: format!("QCOW read length does not fit in a guest offset: {err:?}"),
            })?)
            .ok_or_else(|| AccessorError::Volume {
                reason: String::from("QCOW position does not fit in a guest offset"),
            })?;

        Ok(bytes)
    }

    /// Decompress bytes into `buf`
    fn copy_decompressed(
        &mut self,
        buf: &mut [u8],
        host_offset: u64,
        compressed_size: u64,
        offset_in_cluster: u64,
    ) -> AccessorResult<()> {
        let guest_index = self.qcow.cluster.guest_cluster_index(self.position);
        if self
            .decompress
            .as_ref()
            .is_none_or(|d| d.guest_index != guest_index)
        {
            let src = read_offset_bytes(&mut self.image, host_offset, compressed_size)?;
            let bytes = decompress_cluster(
                self.qcow.header.compression,
                &src,
                self.qcow.cluster.cluster_size,
            )?;

            self.decompress = Some(DecompressCluster { guest_index, bytes });
        }

        let Some(decoded) = &self.decompress else {
            return Err(AccessorError::volume(
                "QCOW compressed cluster was not decompressed",
            ));
        };

        let start = usize::try_from(offset_in_cluster).map_err(|err| {
            AccessorError::volume(format!(
                "QCOW offset inside the cluster does not fit in memory: {err:?}"
            ))
        })?;

        let end = start.checked_add(buf.len()).ok_or_else(|| {
            AccessorError::volume("QCOW read extends past the decompressed cluster")
        })?;

        let Some(slice) = decoded.bytes.get(start..end) else {
            return Err(AccessorError::volume(
                "QCOW read extends past the decompressed cluster",
            ));
        };

        buf.copy_from_slice(slice);

        Ok(())
    }
}

impl<R: Read + Seek> Read for QcowReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_guest(buf)
            .map_err(|err| io::Error::new(ErrorKind::InvalidData, err.to_string()))
    }
}

impl<R: Read + Seek> Seek for QcowReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.seek_guest(pos)
            .map_err(|err| io::Error::new(ErrorKind::InvalidData, err.to_string()))
    }
}

/// Return the size of the QCOW disk image
fn image_length<R: Seek>(image: &mut R) -> AccessorResult<u64> {
    let len = image.seek(SeekFrom::End(0))?;
    image.seek(SeekFrom::Start(0))?;

    Ok(len)
}

/// Read the start of the QCOW disk image
fn read_header<R: Read + Seek>(image: &mut R, file_len: u64) -> AccessorResult<QcowHeader> {
    let version2_header_length = 72;
    if file_len < version2_header_length {
        let buf = read_offset_bytes(image, 0, file_len)?;
        return parse_header(&buf);
    }

    let version3_header_length = 104;
    let probe_length = min(file_len, version3_header_length);

    let mut buf = read_offset_bytes(image, 0, probe_length)?;
    let Ok(version) = read_value(&buf, 4) else {
        return parse_header(&buf);
    };

    if version == 3
        && let Ok(header_length) = read_value(&buf, 100)
    {
        let max_header_length = 1 << 21;
        if header_length > max_header_length {
            return Err(AccessorError::volume(format!(
                "QCOW header length {header_length} exceeds the maximum cluster size"
            )));
        }

        let buffered = u64::try_from(buf.len()).unwrap_or(u64::MAX);
        if header_length as u64 > buffered && header_length as u64 <= file_len {
            buf = read_offset_bytes(image, 0, header_length as u64)?;
        }
    }

    parse_header(&buf)
}

/// Read 4 byte value at provided offset
fn read_value(buf: &[u8], offset: usize) -> AccessorResult<u32> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| AccessorError::volume("QCOW read offset overflows"))?;

    let chunk = buf
        .get(offset..end)
        .ok_or_else(|| AccessorError::volume("QCOW read offset end overflows"))?;

    let (_remaining, offset) = nom_bu32(chunk, "QCOW offset truncated")?;

    Ok(offset)
}

/// Read bytes at provided offset
fn read_offset_bytes<R: Read + Seek>(
    image: &mut R,
    offset: u64,
    length: u64,
) -> AccessorResult<Vec<u8>> {
    let length_usize = usize::try_from(length).map_err(|err| {
        AccessorError::volume(format!(
            "QCOW read of {length} bytes at offset {offset} does not fit in memory: {err:?}"
        ))
    })?;

    let mut buf = vec![0_u8; length_usize];
    image.seek(SeekFrom::Start(offset))?;

    match image.read_exact(&mut buf) {
        Ok(()) => Ok(buf),
        Err(err) if err.kind() == ErrorKind::UnexpectedEof => Err(AccessorError::volume(format!(
            "QCOW read of {length} bytes at offset {offset} extends past the image"
        ))),
        Err(err) => Err(err.into()),
    }
}

/// Validate seek offsets for the guest OS on the QCOW disk image
fn checked_seek(base: u64, delta: i64) -> AccessorResult<u64> {
    if delta >= 0 {
        let delta = u64::try_from(delta)
            .map_err(|err| AccessorError::volume(format!("QCOW seek offset overflows: {err:?}")))?;
        return base
            .checked_add(delta)
            .ok_or_else(|| AccessorError::volume("QCOW seek offset overflows"));
    }

    let backward = delta
        .checked_neg()
        .ok_or_else(|| AccessorError::volume("QCOW seek offset overflows"))?;

    let backward = u64::try_from(backward)
        .map_err(|err| AccessorError::volume(format!("QCOW seek offset overflows: {err:?}")))?;

    base.checked_sub(backward).ok_or_else(|| {
        AccessorError::volume("QCOW seek offset is before the start of the virtual disk")
    })
}

/// Metadata for a QCOW image
///
/// Most data comes from the header
#[derive(Debug, Clone, PartialEq, Serialize)]
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
    /// Flags associated with the QCOW disk
    pub(crate) flags: QcowFeatureFlags,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
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
///
/// <https://www.qemu.org/docs/master/interop/qcow2.html#header>
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
    use super::QcowReader;
    use crate::accessor::error::{AccessorError, AccessorResult};
    use miniz_oxide::{
        DataFormat,
        deflate::core::{CompressionStrategy, CompressorOxide, TDEFLFlush, TDEFLStatus, compress},
    };
    use std::io::{Cursor, ErrorKind, Read, Seek, SeekFrom};

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_be_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_be_bytes());
    }

    fn write_header(
        image: &mut [u8],
        version: u32,
        cluster_bits: u32,
        virtual_size: u64,
        l1_entries: u32,
        l1_offset: u64,
    ) {
        image[..4].copy_from_slice(b"QFI\xfb");
        put_u32(image, 4, version);
        put_u32(image, 20, cluster_bits);
        put_u64(image, 24, virtual_size);

        put_u32(image, 36, l1_entries);
        put_u64(image, 40, l1_offset);

        if version >= 3 {
            put_u32(image, 100, 112);
        }
    }

    fn guest_image(virtual_size: u64, level2_entry: u64, cluster: &[u8]) -> Vec<u8> {
        let mut image = vec![0_u8; 2048];
        write_header(&mut image, 3, 9, virtual_size, 1, 512);

        put_u64(&mut image, 512, 1024);
        put_u64(&mut image, 1024, level2_entry);
        let n = cluster.len().min(512);

        image[1536..1536 + n].copy_from_slice(&cluster[..n]);
        image
    }

    fn pattern() -> Vec<u8> {
        let mut cluster = vec![0x5A; 512];
        cluster[0] = 0x11;
        cluster[100] = 0x22;
        cluster[511] = 0x33;

        cluster
    }

    fn open_image(bytes: Vec<u8>) -> AccessorResult<QcowReader<Cursor<Vec<u8>>>> {
        QcowReader::open(Cursor::new(bytes))
    }

    fn volume_reason<T>(result: AccessorResult<T>) -> String {
        match result {
            Err(AccessorError::Volume { reason }) => reason,
            Err(err) => panic!("expected a volume error, got {err}"),
            Ok(_) => panic!("expected a volume error"),
        }
    }

    fn raw_deflate(plain: &[u8]) -> Vec<u8> {
        let mut compressor =
            CompressorOxide::with_params(DataFormat::Raw, 6, CompressionStrategy::Default, 12);
        let mut output = vec![0_u8; plain.len() + 64];
        let mut input = plain;
        let mut out_pos = 0;

        loop {
            let (status, bytes_in, bytes_out) = compress(
                &mut compressor,
                input,
                &mut output[out_pos..],
                TDEFLFlush::Finish,
            );

            out_pos += bytes_out;

            match status {
                TDEFLStatus::Done => {
                    output.truncate(out_pos);
                    return output;
                }
                TDEFLStatus::Okay => {
                    input = &input[bytes_in..];
                    if output.len() - out_pos < 64 {
                        output.resize(output.len() * 2, 0);
                    }
                }
                other => panic!("raw deflate failed: {other:?}"),
            }
        }
    }

    #[test]
    fn test_read_data_cluster() {
        let cluster = pattern();
        let mut reader = open_image(guest_image(4096, 1536, &cluster)).unwrap();
        let mut empty = [];

        assert_eq!(reader.read(&mut empty).unwrap(), 0);
        assert_eq!(reader.position(), 0);

        let mut buf = [0_u8; 16];
        assert_eq!(reader.read(&mut buf).unwrap(), 16);
        assert_eq!(buf, cluster[..16]);
        assert_eq!(reader.position(), 16);

        assert_ne!(buf[0], b'Q');
        assert_eq!(reader.seek(SeekFrom::Start(100)).unwrap(), 100);

        let mut one = [0_u8; 1];
        assert_eq!(reader.read(&mut one).unwrap(), 1);
        assert_eq!(one[0], 0x22);
    }

    #[test]
    fn test_read_crosses_cluster_and_stops_at_virtual_end() {
        let cluster = pattern();
        let mut reader = open_image(guest_image(600, 1536, &cluster)).unwrap();
        let mut buf = vec![1_u8; 1000];

        assert_eq!(reader.read(&mut buf).unwrap(), 600);
        assert_eq!(&buf[..512], cluster.as_slice());
        assert!(buf[512..600].iter().all(|byte| *byte == 0));

        assert_eq!(reader.position(), 600);
        assert_eq!(reader.read(&mut buf).unwrap(), 0);
    }

    #[test]
    fn test_read_explicit_zero_ignores_host_bytes() {
        let mut reader = open_image(guest_image(512, 1 | 1536, &[0xFF; 512])).unwrap();
        let mut buf = [1_u8; 512];

        assert_eq!(reader.read(&mut buf).unwrap(), 512);
        assert!(buf.iter().all(|byte| *byte == 0));
    }

    #[test]
    fn test_seek_uses_virtual_size() {
        let mut reader = open_image(guest_image(600, 1536, &pattern())).unwrap();
        assert_eq!(reader.virtual_size(), 600);
        assert_eq!(reader.seek(SeekFrom::Start(100)).unwrap(), 100);
        assert_eq!(reader.seek(SeekFrom::Current(5)).unwrap(), 105);

        assert_eq!(reader.seek(SeekFrom::End(0)).unwrap(), 600);
        assert_eq!(reader.seek(SeekFrom::End(-10)).unwrap(), 590);
        assert_eq!(reader.seek(SeekFrom::End(10)).unwrap(), 610);
        assert_eq!(reader.read(&mut [0_u8; 8]).unwrap(), 0);
        assert_eq!(reader.seek(SeekFrom::Start(0)).unwrap(), 0);

        let err = reader.seek(SeekFrom::Current(-1)).unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert!(err.to_string().contains("before the start"), "{err}");
        assert_eq!(reader.position(), 0);

        let err = reader.seek(SeekFrom::End(-601)).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
    }

    #[test]
    fn test_open_allows_dirty_and_corrupt() {
        let mut image = guest_image(512, 1536, &pattern());
        put_u64(&mut image, 72, 0b11);

        let mut reader = open_image(image).unwrap();
        let mut buf = [0_u8; 1];

        assert_eq!(reader.read(&mut buf).unwrap(), 1);
        assert_eq!(buf[0], 0x11);
    }

    #[test]
    fn test_open_rejects_encryption_and_backing_file() {
        let mut encrypted = guest_image(512, 1536, &[]);
        put_u32(&mut encrypted, 32, 1);

        let reason = volume_reason(open_image(encrypted));
        assert!(reason.contains("encryption"), "{reason}");

        let mut backed = guest_image(512, 1536, &[]);
        put_u64(&mut backed, 8, 80);
        put_u32(&mut backed, 16, 4);

        let reason = volume_reason(open_image(backed));
        assert!(reason.contains("backing"), "{reason}");
    }

    #[test]
    fn test_read_damaged_level2_is_invalid_data() {
        let mut image = vec![0_u8; 1024];
        write_header(&mut image, 3, 9, 512, 1, 512);
        put_u64(&mut image, 512, 1024);

        let mut reader = open_image(image).unwrap();
        let err = reader.read(&mut [0_u8; 16]).unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert!(err.to_string().contains("QCOW guest data invalid"), "{err}");
        assert_eq!(reader.position(), 0);
    }

    #[test]
    fn test_read_version2_image() {
        let cluster = pattern();
        let mut image = vec![0_u8; 2048];
        write_header(&mut image, 2, 9, 512, 1, 512);
        put_u64(&mut image, 512, 1024);
        put_u64(&mut image, 1024, 1536);

        image[1536..1536 + cluster.len()].copy_from_slice(&cluster);
        let mut reader = open_image(image).unwrap();
        let mut buf = [0_u8; 16];

        assert_eq!(reader.read(&mut buf).unwrap(), 16);
        assert_eq!(buf, cluster[..16]);
    }

    #[test]
    fn test_open_rejects_short_file() {
        let reason = volume_reason(open_image(vec![0_u8; 10]));
        assert!(reason.contains("QCOW"), "{reason}");
    }

    #[test]
    fn test_read_compressed_cluster() {
        let cluster = pattern();
        let compressed = raw_deflate(&cluster);
        assert!(compressed.len() <= 512, "{}", compressed.len());

        let mut reader = open_image(guest_image(512, 1536 | (1 << 62), &compressed)).unwrap();
        let mut prefix = [0_u8; 16];
        assert_eq!(reader.read(&mut prefix).unwrap(), 16);
        assert_eq!(prefix, cluster[..16]);
        assert_eq!(reader.position(), 16);

        let mut next = [0_u8; 16];
        assert_eq!(reader.read(&mut next).unwrap(), 16);
        assert_eq!(next, cluster[16..32]);
        assert_eq!(reader.seek(SeekFrom::Start(100)).unwrap(), 100);

        let mut one = [0_u8; 1];
        assert_eq!(reader.read(&mut one).unwrap(), 1);
        assert_eq!(one[0], 0x22);
    }

    #[test]
    fn test_read_damaged_compressed_cluster_keeps_position() {
        let mut reader = open_image(guest_image(512, 1536 | (1 << 62), &[])).unwrap();
        let err = reader.read(&mut [0_u8; 16]).unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert!(err.to_string().contains("zlib"), "{err}");
        assert_eq!(reader.position(), 0);
    }

    #[test]
    fn test_read_crosses_into_compressed_cluster() {
        let first = pattern();
        let mut second = vec![0xAB; 512];
        second[0] = 0x44;
        second[511] = 0x55;

        let compressed = raw_deflate(&second);
        assert!(compressed.len() <= 512, "{}", compressed.len());
        let mut image = vec![0_u8; 2560];
        write_header(&mut image, 3, 9, 1024, 1, 512);

        put_u64(&mut image, 512, 1024);
        put_u64(&mut image, 1024, 1536);
        put_u64(&mut image, 1032, 2048 | (1 << 62));

        image[1536..1536 + first.len()].copy_from_slice(&first);
        image[2048..2048 + compressed.len()].copy_from_slice(&compressed);
        let mut reader = open_image(image).unwrap();
        let mut buf = vec![0_u8; 1024];

        assert_eq!(reader.read(&mut buf).unwrap(), 1024);
        assert_eq!(&buf[..512], first.as_slice());
        assert_eq!(&buf[512..], second.as_slice());
        assert_eq!(reader.position(), 1024);
    }

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
