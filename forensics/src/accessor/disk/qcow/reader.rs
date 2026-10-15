use std::{
    cmp::min,
    io::{self, ErrorKind, Read, Seek, SeekFrom},
};
use tracing::warn;

use crate::{
    accessor::{
        disk::qcow::{
            header::{QcowHeader, parse_header},
            level::{GuestMapping, Level2Cache, QcowMap},
        },
        error::{AccessorError, AccessorResult},
    },
    utils::nom_helper::nom_bu32,
};

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

    /// Read guest OS bytes in the QCOW dik image
    fn read_guest(&mut self, buf: &mut [u8]) -> AccessorResult<usize> {
        let virtual_size = self.virtual_size();
        if buf.is_empty() || self.position >= virtual_size {
            return Ok(0);
        }

        let remaining = virtual_size - self.position;
        let required = match usize::try_from(remaining) {
            Ok(result) => result,
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
            .map_err(|err| AccessorError::volume("QCOW guest data invalid"))?;

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
                    return Err(AccessorError::Volume {
                        reason: format!("QCOW host read failed: {err:?}"),
                    });
                }
            }
            GuestMapping::Compressed {
                host_offset,
                compressed_size,
            } => {
                return Err(AccessorError::Volume {
                    reason: format!("QCOW compressed clusters are not supported"),
                });
            }
        }

        self.position = self
            .position
            .checked_add(u64::try_from(bytes).map_err(|err| AccessorError::Volume {
                reason: format!("QCOW read length does not fit in a guest offset: {err:?}"),
            })?)
            .ok_or_else(|| AccessorError::Volume {
                reason: format!("QCOW position does not fit in a guest offset"),
            })?;

        Ok(bytes)
    }
}

impl<R: Read + Seek> Read for QcowReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        let virtual_size = self.virtual_size();
        if buf.is_empty() || self.position >= virtual_size {
            return Ok(0);
        }

        let remaining = virtual_size - self.position;
        let want = match usize::try_from(remaining) {
            Ok(n) => min(n, buf.len()),
            Err(_) => buf.len(),
        };

        let mut filled = 0;
        while filled < want {
            match self.read_cluster(&mut buf[filled..want]) {
                Ok(n) => filled += n,
                Err(_) if filled > 0 => return Ok(filled),
                Err(err) => {
                    return Err(io::Error::new(
                        ErrorKind::InvalidData,
                        format!("Could not read QCOW disk: {err:?}"),
                    ));
                }
            }
        }

        Ok(filled)
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
        let buf = read_prefix(image, file_len)?;
        return parse_header(&buf);
    }

    let version3_header_length = 104;
    let probe_length = min(file_len, version3_header_length);

    let mut buf = read_prefix(image, probe_length)?;
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
            buf = read_prefix(image, header_length as u64)?;
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

/// Read bytes from the QCOW disk image
fn read_prefix<R: Read + Seek>(image: &mut R, length: u64) -> AccessorResult<Vec<u8>> {
    let length_usize = usize::try_from(length).map_err(|_| {
        AccessorError::volume(format!(
            "QCOW header is {length} bytes and does not fit in memory"
        ))
    })?;

    let mut buf = vec![0_u8; length_usize];
    image.seek(SeekFrom::Start(0))?;

    match image.read_exact(&mut buf) {
        Ok(()) => Ok(buf),
        Err(err) if err.kind() == ErrorKind::UnexpectedEof => Err(AccessorError::volume(format!(
            "QCOW header is shorter than {length} bytes"
        ))),
        Err(err) => Err(err.into()),
    }
}

/// Validate seek offsets for the guest OS on the QCOW disk image
fn checked_seek(base: u64, delta: i64) -> AccessorResult<u64> {
    if delta >= 0 {
        let delta = u64::try_from(delta)
            .map_err(|_| AccessorError::volume("QCOW seek offset overflows"))?;
        return base
            .checked_add(delta)
            .ok_or_else(|| AccessorError::volume("QCOW seek offset overflows"));
    }

    let backward = delta
        .checked_neg()
        .ok_or_else(|| AccessorError::volume("QCOW seek offset overflows"))?;

    let backward =
        u64::try_from(backward).map_err(|_| AccessorError::volume("QCOW seek offset overflows"))?;

    base.checked_sub(backward).ok_or_else(|| {
        AccessorError::volume("QCOW seek offset is before the start of the virtual disk")
    })
}

#[cfg(test)]
mod tests {
    use super::QcowReader;
    use crate::accessor::error::{AccessorError, AccessorResult};
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
    fn test_read_compressed_cluster_errors() {
        let image = guest_image(512, 1536 | (1 << 62), &[]);
        let mut reader = open_image(image).unwrap();
        let err = reader.read(&mut [0_u8; 16]).unwrap_err();

        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert!(err.to_string().contains("compressed"), "{err}");
        assert_eq!(reader.position(), 0);
    }

    #[test]
    fn test_read_returns_bytes_before_compressed_cluster() {
        let cluster = pattern();
        let mut image = vec![0_u8; 2560];
        write_header(&mut image, 3, 9, 1024, 1, 512);

        put_u64(&mut image, 512, 1024);
        put_u64(&mut image, 1024, 1536);
        put_u64(&mut image, 1032, 2048 | (1 << 62));
        image[1536..1536 + cluster.len()].copy_from_slice(&cluster);

        let mut reader = open_image(image).unwrap();
        let mut buf = vec![0_u8; 800];

        assert_eq!(reader.read(&mut buf).unwrap(), 512);
        assert_eq!(&buf[..512], cluster.as_slice());
        assert_eq!(reader.position(), 512);

        let err = reader.read(&mut buf).unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert!(err.to_string().contains("compressed"), "{err}");
        assert_eq!(reader.position(), 512);

        assert_eq!(reader.seek(SeekFrom::Start(0)).unwrap(), 0);
        let mut prefix = [0_u8; 16];

        assert_eq!(reader.read(&mut prefix).unwrap(), 16);
        assert_eq!(prefix, cluster[..16]);
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
}
