use crate::accessor::{
    disk::{
        inspect::read_at,
        vhdx::{
            bat::{VhdxBat, VhdxMapping, parse_bat},
            header::{VhdxHeader, active_head},
            identifier::parse_header,
            metadata::{VhdxMetadata, parse_metadata},
            region::{VhdxRegion, VhdxRegionTable, active_regions},
        },
    },
    error::{AccessorError, AccessorResult},
};
use std::{
    cmp::min,
    io::{self, ErrorKind, Read, Seek, SeekFrom},
};

/// Guest reader for a VHDX file
///
/// Reads the internal guest of a VHDX file
#[derive(Debug)]
pub(crate) struct VhdxReader<R> {
    /// Reader for the VHDX file
    image: R,
    /// Position of the VHDX guest
    position: u64,
    /// Size of the VHDX file
    file_len: u64,
    /// Parsed VHDX metadata
    metadata: VhdxMetadata,
    /// Parsed VHDX BAT
    bat: VhdxBat,
    /// Creator application of the VHDX file
    creator: String,
}

impl<R: Read + Seek> VhdxReader<R> {
    /// Open a VHDX file and start the guest reader at offset 0
    pub(super) fn open(mut image: R) -> AccessorResult<Self> {
        let min_length = 256 * 1024 + 64 * 1024;
        let file_len = image_size(&mut image)?;
        if file_len < min_length {
            return Err(AccessorError::volume(format!(
                "VHDX file is {file_len} bytes. Wanted at least {min_length}"
            )));
        }

        let identifier = parse_header(&read_at(&mut image, 0, 520)?)?;

        let header1_offset = 64 * 1024;
        let header2_offset = 128 * 1024;
        let header_length = 4096;
        let header = active_head(
            &read_at(&mut image, header1_offset, header_length)?,
            &read_at(&mut image, header2_offset, header_length)?,
        )?;

        let region1_offset = 192 * 1024;
        let region2_offset = 256 * 1024;
        let region_length = 64 * 1024;
        let regions = active_regions(
            &read_at(&mut image, region1_offset, region_length)?,
            &read_at(&mut image, region2_offset, region_length)?,
        )?;

        region_in_file("metadata", &regions.metadata, file_len)?;
        region_in_file("BAT", &regions.bat, file_len)?;

        let log = log_range(&header, &regions, file_len)?;

        let metadata = parse_metadata(&read_at(
            &mut image,
            regions.metadata.file_offset,
            regions.metadata.length as usize,
        )?)?;

        let bat = parse_bat(
            &read_at(
                &mut image,
                regions.bat.file_offset,
                regions.bat.length as usize,
            )?,
            &metadata,
            file_len,
            &occupied_ranges(&regions, log),
        )?;

        Ok(Self {
            image,
            position: 0,
            file_len,
            metadata,
            bat,
            creator: identifier.creator(),
        })
    }

    /// Size of the virtual disk in bytes
    pub(crate) fn virtual_size(&self) -> u64 {
        self.metadata.virtual_disk_size
    }

    /// Size of the VHDX file in bytes
    pub(crate) fn file_size(&self) -> u64 {
        self.file_len
    }

    /// Logical sector size in bytes. Will be 512 or 4096
    pub(crate) fn logical_sector_size(&self) -> u32 {
        self.metadata.logical_sector_size
    }

    /// Physical sector size in bytes. Will be 512 or 4096
    pub(crate) fn physical_sector_size(&self) -> u32 {
        self.metadata.physical_sector_size
    }

    /// Position of the reader in VHDX guest
    pub(crate) fn position(&self) -> u64 {
        self.position
    }

    /// Application that created the VHDX file.
    pub(crate) fn creator(&self) -> &str {
        &self.creator
    }

    /// Read bytes in the VHDX guest
    fn read_guest(&mut self, buf: &mut [u8]) -> AccessorResult<usize> {
        if buf.is_empty() || self.position >= self.virtual_size() {
            return Ok(0);
        }

        let remaining = self.virtual_size() - self.position;
        let want = match usize::try_from(remaining) {
            Ok(remaining) => min(remaining, buf.len()),
            Err(_) => buf.len(),
        };

        let mut filled = 0;
        while filled < want {
            match self.read_mapping(&mut buf[filled..want]) {
                Ok(read) => filled += read,
                Err(_) if filled > 0 => return Ok(filled),
                Err(err) => return Err(err),
            }
        }

        Ok(filled)
    }

    /// Move the reader in the VHDX guest
    fn seek_guest(&mut self, pos: SeekFrom) -> AccessorResult<u64> {
        let next = match pos {
            SeekFrom::Start(offset) => offset,
            SeekFrom::Current(delta) => checked_seek(self.position, delta)?,
            SeekFrom::End(delta) => checked_seek(self.virtual_size(), delta)?,
        };

        self.position = next;
        Ok(self.position)
    }

    /// Read bytes from BAT from the VHDX file
    fn read_mapping(&mut self, buf: &mut [u8]) -> AccessorResult<usize> {
        let mapping = self.bat.map_offset(self.position)?;
        let (file_offset, length) = match mapping {
            VhdxMapping::Zero { length } => (None, length),
            VhdxMapping::Data {
                file_offset,
                length,
            } => (Some(file_offset), length),
        };

        let available = usize::try_from(length).unwrap_or(usize::MAX);
        let bytes = min(buf.len(), available);
        if bytes == 0 {
            return Err(AccessorError::volume(
                "VHDX read returned no bytes inside the virtual disk",
            ));
        }

        if let Some(file_offset) = file_offset {
            self.image.seek(SeekFrom::Start(file_offset))?;
            if let Err(err) = self.image.read_exact(&mut buf[..bytes]) {
                return Err(AccessorError::Io {
                    path: None,
                    source: err,
                });
            }
        } else {
            buf[..bytes].fill(0);
        }

        let advance = u64::try_from(bytes).map_err(|err| {
            AccessorError::volume(format!(
                "VHDX read length does not fit in a guest offset: {err:?}"
            ))
        })?;

        self.position = self
            .position
            .checked_add(advance)
            .ok_or_else(|| AccessorError::volume("VHDX position does not fit in a guest offset"))?;

        Ok(bytes)
    }
}

impl<R: Read + Seek> Read for VhdxReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.read_guest(buf)
            .map_err(|err| io::Error::new(ErrorKind::InvalidData, err.to_string()))
    }
}

impl<R: Read + Seek> Seek for VhdxReader<R> {
    fn seek(&mut self, pos: SeekFrom) -> io::Result<u64> {
        self.seek_guest(pos)
            .map_err(|err| io::Error::new(ErrorKind::InvalidData, err.to_string()))
    }
}

/// Return size of the VHDX file
fn image_size<R: Seek>(image: &mut R) -> AccessorResult<u64> {
    let len = image.seek(SeekFrom::End(0))?;
    image.seek(SeekFrom::Start(0))?;

    Ok(len)
}

/// Validate `VhdxRegion` values are within the file size
fn region_in_file(name: &str, region: &VhdxRegion, file_len: u64) -> AccessorResult<()> {
    let length = region.length as u64;
    let end = region
        .file_offset
        .checked_add(length)
        .ok_or_else(|| AccessorError::volume(format!("VHDX {name} region range overflows")))?;

    if end > file_len {
        return Err(AccessorError::volume(format!(
            "VHDX {name} region at offset {} length {length} extends past the file",
            region.file_offset
        )));
    }

    Ok(())
}

/// Return log range if a log value is found
fn log_range(
    header: &VhdxHeader,
    regions: &VhdxRegionTable,
    file_len: u64,
) -> AccessorResult<Option<(u64, u64)>> {
    let length = header.log_length as u64;
    if length == 0 {
        return Ok(None);
    }

    let offset = header.log_offset;
    if offset < 1048576 || !offset.is_multiple_of(1048576) || !length.is_multiple_of(1048576) {
        return Err(AccessorError::volume(format!(
            "VHDX log at offset {offset} length {length} is not 1 MiB aligned"
        )));
    }

    let end = offset
        .checked_add(length)
        .ok_or_else(|| AccessorError::volume("VHDX log range overflows"))?;
    if end > file_len {
        return Err(AccessorError::volume(format!(
            "VHDX log at offset {offset} length {length} extends past the file"
        )));
    }

    if overlaps(
        offset,
        length,
        regions.bat.file_offset,
        regions.bat.length as u64,
    ) {
        return Err(AccessorError::volume("VHDX log overlaps the BAT region"));
    }

    if overlaps(
        offset,
        length,
        regions.metadata.file_offset,
        regions.metadata.length as u64,
    ) {
        return Err(AccessorError::volume(
            "VHDX log overlaps the metadata region",
        ));
    }

    Ok(Some((offset, length)))
}

/// Make sure BAT value does not overlap other parsed sections
fn occupied_ranges(regions: &VhdxRegionTable, log: Option<(u64, u64)>) -> Vec<(u64, u64)> {
    let mut ranges = vec![
        (regions.metadata.file_offset, regions.metadata.length as u64),
        (regions.bat.file_offset, regions.bat.length as u64),
    ];
    if let Some(log) = log {
        ranges.push(log);
    }

    ranges
}

/// Compare two byte ranges
fn overlaps(start: u64, length: u64, other: u64, other_length: u64) -> bool {
    let Some(end) = start.checked_add(length) else {
        return true;
    };

    let Some(other_end) = other.checked_add(other_length) else {
        return true;
    };

    start < other_end && other < end
}

/// Validate seeks in the guest VHDX
fn checked_seek(base: u64, delta: i64) -> AccessorResult<u64> {
    if delta >= 0 {
        let delta = u64::try_from(delta)
            .map_err(|err| AccessorError::volume(format!("VHDX seek offset overflows: {err:?}")))?;

        return base
            .checked_add(delta)
            .ok_or_else(|| AccessorError::volume("VHDX seek offset overflows"));
    }

    let backward = delta
        .checked_neg()
        .ok_or_else(|| AccessorError::volume("VHDX seek offset overflows"))?;
    let backward = u64::try_from(backward)
        .map_err(|err| AccessorError::volume(format!("VHDX seek offset overflows: {err:?}")))?;

    base.checked_sub(backward).ok_or_else(|| {
        AccessorError::volume("VHDX seek offset is before the start of the virtual disk")
    })
}

#[cfg(test)]
mod test {
    use super::VhdxReader;
    use crate::accessor::{
        disk::vhdx::header::vhdx_crc32c,
        error::{AccessorError, AccessorResult},
    };
    use std::{
        fs,
        io::{Cursor, ErrorKind, Read, Seek, SeekFrom},
        path::PathBuf,
    };
    use uuid::Uuid;

    fn put_u16(bytes: &mut [u8], offset: usize, value: u16) {
        bytes[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn put_guid(bytes: &mut [u8], offset: usize, guid: Uuid) {
        bytes[offset..offset + 16].copy_from_slice(&guid.to_bytes_le());
    }

    fn bat_entry(state: u64, file_offset: u64) -> u64 {
        (file_offset & !0xF_FFFF) | (state & 7)
    }

    fn guest_image(
        blocks: &[Option<u8>],
        virtual_size: u64,
        fixed: bool,
        log_offset: u64,
        pending_log: bool,
    ) -> Vec<u8> {
        let mut file_len = 3145728 + 1048576;
        file_len = file_len.max(log_offset + 1048576);
        for (index, block) in blocks.iter().enumerate() {
            if block.is_some() {
                let end = 4194304 + index as u64 * 1048576 + 1048576;
                file_len = file_len.max(end);
            }
        }

        let mut image = vec![0xFF; usize::try_from(file_len).unwrap()];
        write_identifier(&mut image);
        write_header(&mut image, 64 * 1024, 1, log_offset, pending_log);
        write_header(&mut image, 128 * 1024, 2, log_offset, pending_log);

        let regions = region_table();
        image[192 * 1024..192 * 1024 + 65536].copy_from_slice(&regions);
        image[256 * 1024..256 * 1024 + 65536].copy_from_slice(&regions);

        write_metadata(&mut image, virtual_size, fixed);
        write_bat(&mut image, blocks, fixed);

        for (index, block) in blocks.iter().enumerate() {
            let Some(byte) = block else {
                continue;
            };
            let start = usize::try_from(4194304 + index as u64 * 1048576).unwrap();
            image[start..start + usize::try_from(1048576).unwrap()].fill(*byte);
        }

        image
    }

    fn write_identifier(image: &mut [u8]) {
        image[..8].copy_from_slice(b"vhdxfile");
        image[8..8 + 512].fill(0);
        for (index, unit) in "unit".encode_utf16().enumerate() {
            let start = 8 + index * 2;
            image[start..start + 2].copy_from_slice(&unit.to_le_bytes());
        }
    }

    fn write_header(
        image: &mut [u8],
        offset: usize,
        sequence: u64,
        log_offset: u64,
        pending: bool,
    ) {
        let mut bytes = vec![0_u8; 4096];
        put_u32(&mut bytes, 0, 0x64616568);
        put_u64(&mut bytes, 8, sequence);
        put_guid(&mut bytes, 16, Uuid::from_u128(1));
        put_guid(&mut bytes, 32, Uuid::from_u128(2));

        let log = if pending {
            Uuid::from_u128(3)
        } else {
            Uuid::nil()
        };

        put_guid(&mut bytes, 48, log);
        put_u16(&mut bytes, 64, 0);
        put_u16(&mut bytes, 66, 1);

        put_u32(&mut bytes, 68, 1048576);
        put_u64(&mut bytes, 72, log_offset);
        let checksum = vhdx_crc32c(&bytes, 4096);
        put_u32(&mut bytes, 4, checksum);
        image[offset..offset + 4096].copy_from_slice(&bytes);
    }

    fn region_table() -> Vec<u8> {
        let mut bytes = vec![0_u8; 65536];
        put_u32(&mut bytes, 0, 0x69676572);
        put_u32(&mut bytes, 8, 2);

        put_guid(
            &mut bytes,
            16,
            Uuid::from_u128(0x2dc27766f62342009d64115e9bfd4a08),
        );

        put_u64(&mut bytes, 32, 2097152);
        put_u32(&mut bytes, 40, 1048576 as u32);
        put_u32(&mut bytes, 44, 0);

        put_guid(
            &mut bytes,
            48,
            Uuid::from_u128(0x8b7ca20647904b9ab8fe575f050f886e),
        );
        put_u64(&mut bytes, 64, 3145728);
        put_u32(&mut bytes, 72, 1048576 as u32);
        put_u32(&mut bytes, 76, 0);

        let checksum = vhdx_crc32c(&bytes, 65536);
        put_u32(&mut bytes, 4, checksum);
        bytes
    }

    fn write_metadata(image: &mut [u8], virtual_size: u64, fixed: bool) {
        let start = usize::try_from(3145728).unwrap();
        image[start..start + 0x10030].fill(0);
        put_u64(image, start, 0x617461646174656d);
        put_u16(image, start + 10, 5);

        let items = [
            (
                Uuid::from_u128(0xcaa16737fa364d43b3b633f0aa44e76b),
                0x10000_u32,
                8_u32,
            ),
            (
                Uuid::from_u128(0x2fa54224cd1b4876b2115dbed83bf4b8),
                0x10008,
                8,
            ),
            (
                Uuid::from_u128(0xbeca12abb2e6452393efc309e000c746),
                0x10010,
                16,
            ),
            (
                Uuid::from_u128(0x8141bf1da96f4709ba47f233a8faab5f),
                0x10020,
                4,
            ),
            (
                Uuid::from_u128(0xcda348c7445d44719cc9e9885251c556),
                0x10024,
                4,
            ),
        ];
        for (index, (guid, offset, length)) in items.iter().enumerate() {
            let entry = start + 32 + index * 32;
            put_guid(image, entry, *guid);
            put_u32(image, entry + 16, *offset);
            put_u32(image, entry + 20, *length);
            put_u32(image, entry + 24, 0x6);
        }

        let params = start + 0x10000;
        put_u32(image, params, 1048576 as u32);
        put_u32(image, params + 4, fixed as u32);
        put_u64(image, start + 0x10008, virtual_size);

        put_guid(image, start + 0x10010, Uuid::nil());
        put_u32(image, start + 0x10020, 512);
        put_u32(image, start + 0x10024, 512);
    }

    fn write_bat(image: &mut [u8], blocks: &[Option<u8>], fixed: bool) {
        let start = usize::try_from(2097152).unwrap();
        for (index, block) in blocks.iter().enumerate() {
            let entry = match block {
                Some(_) => bat_entry(6, 4194304 + index as u64 * 1048576),
                None => bat_entry(0, 0),
            };
            put_u64(image, start + index * 8, entry);
        }

        if !fixed {
            put_u64(image, start + blocks.len() * 8, bat_entry(0, 0));
        }
    }

    fn open_image(bytes: Vec<u8>) -> AccessorResult<VhdxReader<Cursor<Vec<u8>>>> {
        VhdxReader::open(Cursor::new(bytes))
    }

    fn volume_reason<T>(result: AccessorResult<T>) -> String {
        match result {
            Err(AccessorError::Volume { reason }) => reason,
            Err(err) => panic!("expected a volume error, got {err}"),
            Ok(_) => panic!("expected a volume error"),
        }
    }

    #[test]
    fn test_open_fixture() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/disks/vhdx/ntfs.vhdx");
        let mut reader = open_image(fs::read(path).unwrap()).unwrap();

        assert_eq!(reader.creator(), "QEMU v11.1.0");
        assert_eq!(reader.virtual_size(), 8000000);
        assert_eq!(reader.file_size(), 16 * 1024 * 1024);
        assert_eq!(reader.logical_sector_size(), 512);
        assert_eq!(reader.physical_sector_size(), 512);
        assert_eq!(reader.position(), 0);

        let mut sector = [0_u8; 512];
        reader.read_exact(&mut sector).unwrap();
        assert_eq!(&sector[3..11], b"NTFS    ");
        assert_eq!(reader.position(), 512);
    }

    #[test]
    fn test_read_crosses_block_and_stops_at_virtual_end() {
        let image = guest_image(
            &[Some(0x11), Some(0x22)],
            1048576 + 512,
            false,
            1048576,
            false,
        );

        let mut reader = open_image(image).unwrap();
        assert_eq!(reader.creator(), "unit");
        assert_eq!(reader.virtual_size(), 1048576 + 512);

        reader.seek(SeekFrom::Start(1048576 - 8)).unwrap();
        let mut buf = [0_u8; 1024];
        let read = reader.read(&mut buf).unwrap();

        assert_eq!(read, 520);
        assert_eq!(&buf[..8], &[0x11; 8]);
        assert_eq!(&buf[8..520], &[0x22; 512]);
        assert_eq!(reader.position(), 1048576 + 512);
        assert_eq!(reader.read(&mut buf).unwrap(), 0);
    }

    #[test]
    fn test_read_zero_block_ignores_host_bytes() {
        let image = guest_image(&[None, None], 1048576 * 2, false, 1048576, false);
        let mut reader = open_image(image).unwrap();
        let mut buf = [1_u8; 32];

        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [0_u8; 32]);

        reader.seek(SeekFrom::Start(1048576 + 10)).unwrap();
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [0_u8; 32]);
        assert_eq!(reader.position(), 1048576 + 42);
    }

    #[test]
    fn test_read_zero_then_data() {
        let image = guest_image(&[None, Some(0x22)], 1048576 * 2, false, 1048576, false);
        let mut reader = open_image(image).unwrap();
        reader.seek(SeekFrom::Start(1048576 - 4)).unwrap();

        let mut buf = [0_u8; 8];
        reader.read_exact(&mut buf).unwrap();
        assert_eq!(buf, [0, 0, 0, 0, 0x22, 0x22, 0x22, 0x22]);
        assert_eq!(reader.position(), 1048576 + 4);
    }

    #[test]
    fn test_read_fixed_disk() {
        let image = guest_image(&[Some(0x44)], 1048576, true, 1048576, false);
        let mut reader = open_image(image).unwrap();
        let mut byte = [0_u8; 1];

        reader.read_exact(&mut byte).unwrap();
        assert_eq!(byte, [0x44]);

        reader.seek(SeekFrom::End(-1)).unwrap();
        reader.read_exact(&mut byte).unwrap();
        assert_eq!(byte, [0x44]);
        assert_eq!(reader.position(), 1048576);
    }

    #[test]
    fn test_seek_uses_virtual_size() {
        let image = guest_image(&[Some(0x11)], 1048576, false, 1048576, false);
        let mut reader = open_image(image).unwrap();

        assert_eq!(reader.seek(SeekFrom::Start(10)).unwrap(), 10);
        assert_eq!(reader.seek(SeekFrom::Current(5)).unwrap(), 15);
        assert_eq!(reader.seek(SeekFrom::End(0)).unwrap(), 1048576);

        assert_eq!(reader.seek(SeekFrom::End(-8)).unwrap(), 1048576 - 8);
        assert_eq!(reader.read(&mut []).unwrap(), 0);
        assert_eq!(reader.position(), 1048576 - 8);

        let err = reader
            .seek(SeekFrom::Current(-i64::try_from(1048576).unwrap()))
            .unwrap_err();
        assert_eq!(err.kind(), ErrorKind::InvalidData);
        assert!(err.to_string().contains("before the start"));
        assert_eq!(reader.position(), 1048576 - 8);

        reader.seek(SeekFrom::Start(1048576 + 10)).unwrap();
        let mut buf = [0_u8; 4];
        assert_eq!(reader.read(&mut buf).unwrap(), 0);
    }

    #[test]
    fn test_open_allows_pending_log() {
        let image = guest_image(&[Some(0x11)], 1048576, false, 1048576, true);
        let mut reader = open_image(image).unwrap();
        let mut byte = [0_u8; 1];

        reader.read_exact(&mut byte).unwrap();
        assert_eq!(byte, [0x11]);
    }

    #[test]
    fn test_open_rejects_short_file_and_log_overlap() {
        let reason = volume_reason(open_image(vec![0_u8; 100]));
        assert!(reason.contains("at least"), "{reason}");

        let reason = volume_reason(open_image(vec![0_u8; 320 * 1024]));
        assert!(reason.contains("signature"), "{reason}");

        let image = guest_image(&[Some(0x11)], 1048576, false, 2097152, false);
        let reason = volume_reason(open_image(image));
        assert!(reason.contains("overlaps the BAT"), "{reason}");
    }
}
