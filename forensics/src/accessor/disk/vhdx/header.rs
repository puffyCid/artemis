use crate::{
    accessor::error::{AccessorError, AccessorResult},
    utils::{
        checksum::crc32c_check,
        nom_helper::{nom_guid, nom_u16, nom_u32, nom_u64},
    },
};
use tracing::warn;
use uuid::Uuid;

/// Parsed VHDX Image header
///
/// Two copies are stored in a VHDX file
pub(super) struct VhdxHeader {
    /// Sequence number for the image header
    sequence: u64,
    /// File write ID
    file_write: Uuid,
    /// Data write ID
    data_write: Uuid,
    /// Log ID
    log: Uuid,
    /// Log format version
    log_version: u16,
    /// VHDX format version. Only version 1 has been seen
    version: u16,
    /// Log size
    ///
    /// Will be a multitude of 1 MB
    log_length: u32,
    /// Offset to the log
    ///
    /// Will be a multitude of 1 MB
    log_offset: u64,
}

impl VhdxHeader {
    /// Return if there pending log data to replay
    pub(super) fn has_pending_log(&self) -> bool {
        !self.log.is_nil()
    }
}

/// Parse the VHDX image header
pub(super) fn parse_image_header(data: &[u8]) -> AccessorResult<VhdxHeader> {
    let head_length = 4096;

    if data.len() < head_length {
        return Err(AccessorError::volume(format!(
            "VHDX image header is {} bytes. Wanted at least {head_length}",
            data.len()
        )));
    }

    let (input, sig) = nom_u32(data, "VHDX image header is truncated")?;
    let magic = 0x64616568;
    if sig != magic {
        return Err(AccessorError::volume(format!(
            "VHDX image head signature wrong. Wanted {magic:#0x}, got: {sig:#0x}",
        )));
    }

    let (input, checksum) = nom_u32(input, "VHDX image header checksum truncated")?;

    let block = &data[..head_length];
    let value = vhdx_crc32c(block);
    if checksum != value {
        return Err(AccessorError::volume(format!(
            "VHDX image head checksum incorrect. Wanted {value:#0x}, got: {checksum:#0x}",
        )));
    }

    let (input, sequence) = nom_u64(input, "VHDX image header sequence number truncated")?;
    let (input, file_write) = nom_guid(input, "VHDX image header file write GUID truncated")?;
    let (input, data_write) = nom_guid(input, "VHDX image header data write GUID truncated")?;
    let (input, log) = nom_guid(input, "VHDX image header log GUID truncated")?;

    let (input, log_version) = nom_u16(input, "VHDX image header log version truncated")?;
    let (input, version) = nom_u16(input, "VHDX image header version truncated")?;
    let (input, log_length) = nom_u32(input, "VHDX image header log length truncated")?;
    let (_, log_offset) = nom_u64(input, "VHDX image header log offset truncated")?;

    if version != 1 {
        return Err(AccessorError::volume(format!(
            "VHDX image header version {version} is not supported"
        )));
    }

    if log_version != 0 {
        return Err(AccessorError::volume(format!(
            "VHDX image header log version {log_version} is not supported"
        )));
    }

    Ok(VhdxHeader {
        sequence,
        file_write,
        data_write,
        log,
        log_version,
        version,
        log_length,
        log_offset,
    })
}

/// Return the active image header
///
/// The image header with the higher sequence count is current image we should use
pub(super) fn active_head(first: &[u8], second: &[u8]) -> AccessorResult<VhdxHeader> {
    let selected = match (parse_image_header(first), parse_image_header(second)) {
        (Ok(first_head), Ok(second_head)) if first_head.sequence == second_head.sequence => {
            return Err(AccessorError::volume(
                "VHDX image header copies have the same sequence number",
            ));
        }
        (Ok(first_head), Ok(second_head)) => {
            if first_head.sequence > second_head.sequence {
                first_head
            } else {
                second_head
            }
        }
        (Ok(first_head), Err(_)) => first_head,
        (Err(_), Ok(second_head)) => second_head,
        (Err(first_err), Err(second_err)) => {
            return Err(AccessorError::volume(format!(
                "Both VHDX image header copies are invalid: {first_err:?}; {second_err:?}"
            )));
        }
    };

    if selected.has_pending_log() {
        warn!("VHDX file has pending log");
    }

    Ok(selected)
}

/// Return CRC32C checksum for image head
fn vhdx_crc32c(data: &[u8]) -> u32 {
    let length = 4096;
    let mut bytes = data[..length].to_vec();
    bytes[4..8].fill(0);

    !crc32c_check(0xffffffff, &bytes)
}

#[cfg(test)]
mod test {
    use super::{active_head, parse_image_header, vhdx_crc32c};
    use crate::accessor::error::{AccessorError, AccessorResult};
    use std::{fs::read, path::PathBuf};
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

    fn head_bytes(sequence: u64, log: Uuid, version: u16, log_version: u16) -> Vec<u8> {
        let file_write = Uuid::parse_str("472e3dc8-6f76-f940-8ae2-ff16eb670b4b").unwrap();
        let data_write = Uuid::parse_str("f4cea0e7-79f7-7e49-969e-cb9a2df3c9d6").unwrap();
        let mut bytes = vec![0_u8; 4096];

        put_u32(&mut bytes, 0, 0x64616568);
        put_u64(&mut bytes, 8, sequence);

        put_guid(&mut bytes, 16, file_write);
        put_guid(&mut bytes, 32, data_write);
        put_guid(&mut bytes, 48, log);

        put_u16(&mut bytes, 64, log_version);
        put_u16(&mut bytes, 66, version);
        put_u32(&mut bytes, 68, 1048576);
        put_u64(&mut bytes, 72, 0x100000);

        let checksum = vhdx_crc32c(&bytes);
        put_u32(&mut bytes, 4, checksum);
        bytes
    }

    fn volume_reason<T>(result: AccessorResult<T>) -> String {
        match result {
            Err(AccessorError::Volume { reason }) => reason,
            Err(err) => panic!("expected a volume error, got {err}"),
            Ok(_) => panic!("expected a volume error"),
        }
    }

    fn fixture_copy(offset: u64) -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/disks/vhdx/ntfs.vhdx");

        let bytes = read(path).unwrap();
        let start = usize::try_from(offset).unwrap();
        bytes[start..start + 4096].to_vec()
    }

    #[test]
    fn test_parse_image_header() {
        let bytes = head_bytes(7, Uuid::nil(), 1, 0);
        let head = parse_image_header(&bytes).unwrap();

        assert_eq!(head.sequence, 7);
        assert_eq!(head.version, 1);
        assert_eq!(head.log_version, 0);

        assert_eq!(head.log_length, 1_048_576);
        assert_eq!(head.log_offset, 0x100000);
        assert!(!head.has_pending_log());
        assert_eq!(
            head.file_write,
            Uuid::parse_str("472e3dc8-6f76-f940-8ae2-ff16eb670b4b").unwrap()
        );
    }

    #[test]
    fn test_parse_image_header_rejects_short_bad_signature_and_version() {
        let reason = volume_reason(parse_image_header(&[0_u8; 100]));
        assert!(reason.contains("at least"), "{reason}");

        let mut bytes = head_bytes(1, Uuid::nil(), 1, 0);
        bytes[0] = 0;
        let reason = volume_reason(parse_image_header(&bytes));
        assert!(reason.contains("signature"), "{reason}");

        let reason = volume_reason(parse_image_header(&head_bytes(1, Uuid::nil(), 2, 0)));
        assert!(reason.contains("version 2"), "{reason}");

        let reason = volume_reason(parse_image_header(&head_bytes(1, Uuid::nil(), 1, 1)));
        assert!(reason.contains("log version 1"), "{reason}");
    }

    #[test]
    fn test_active_head_uses_higher_sequence() {
        let first = head_bytes(1, Uuid::nil(), 1, 0);
        let second = head_bytes(2, Uuid::nil(), 1, 0);

        let head = active_head(&first, &second).unwrap();
        assert_eq!(head.sequence, 2);

        let head = active_head(&second, &first).unwrap();
        assert_eq!(head.sequence, 2);
    }

    #[test]
    fn test_active_head_ignores_one_bad_copy() {
        let mut first = head_bytes(9, Uuid::nil(), 1, 0);
        first[100] ^= 0xFF;
        let second = head_bytes(4, Uuid::nil(), 1, 0);

        let head = active_head(&first, &second).unwrap();
        assert_eq!(head.sequence, 4);
    }

    #[test]
    fn test_active_head_rejects_equal_sequence_and_both_invalid() {
        let first = head_bytes(5, Uuid::nil(), 1, 0);
        let second = head_bytes(5, Uuid::nil(), 1, 0);
        let reason = volume_reason(active_head(&first, &second));
        assert!(reason.contains("same sequence"), "{reason}");

        let reason = volume_reason(active_head(&[0_u8; 16], &[0_u8; 16]));
        assert!(
            reason.contains("VHDX image header is 16 bytes. Wanted at least 4096"),
            "{reason}"
        );
    }

    #[test]
    fn test_active_head_allows_pending_log() {
        let log = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        let bytes = head_bytes(3, log, 1, 0);
        let head = active_head(&bytes, &head_bytes(1, log, 1, 0)).unwrap();

        assert!(head.has_pending_log());
        assert_eq!(head.log, log);
        assert_eq!(head.sequence, 3);
    }

    #[test]
    fn test_active_head_fixture() {
        let first = fixture_copy(65536);
        let second = fixture_copy(131072);
        let head = active_head(&first, &second).unwrap();

        assert_eq!(head.sequence, 2_746_793_878);
        assert_eq!(head.version, 1);
        assert_eq!(head.log_version, 0);

        assert_eq!(head.log_offset, 0x100000);
        assert_eq!(head.log_length, 1_048_576);
        assert!(!head.has_pending_log());
        assert_eq!(
            head.file_write,
            Uuid::parse_str("472e3dc8-6f76-f940-8ae2-ff16eb670b4b").unwrap()
        );
        assert_eq!(
            head.data_write,
            Uuid::parse_str("f4cea0e7-79f7-7e49-969e-cb9a2df3c9d6").unwrap()
        );
    }
}
