use crate::{
    accessor::{
        disk::vhdx::{bat::ranges_overlap, header::vhdx_crc32c},
        error::{AccessorError, AccessorResult},
    },
    utils::nom_helper::{nom_guid, nom_u32, nom_u64},
};
use uuid::Uuid;

/// Parse VHDX Region table entry
#[derive(Debug, Clone, PartialEq)]
pub(super) struct VhdxRegion {
    /// Region ID
    pub(super) guid: Uuid,
    /// Offset to the region
    ///
    /// Multitude of 1MB
    pub(super) file_offset: u64,
    /// Length of region in bytes
    ///
    /// Multitude of 1MB
    pub(super) length: u32,
    /// Determine if the region is required
    pub(super) required: bool,
}

/// Parsed locations from the active region table
///
/// Two copies exist
#[derive(Debug, Clone, PartialEq)]
pub(super) struct VhdxRegionTable {
    /// Block allocation table
    ///
    /// Contains the blocks containing a sector bitmap
    pub(super) bat: VhdxRegion,
    /// Parsed VHDX metadata region
    pub(super) metadata: VhdxRegion,
}

/// Parse the VHDX region table
pub(super) fn parse_region_table(data: &[u8]) -> AccessorResult<Vec<VhdxRegion>> {
    let region_size = 65536;
    if data.len() < region_size {
        return Err(AccessorError::volume(format!(
            "VHDX region table is {} bytes. Wanted at least {region_size}",
            data.len()
        )));
    }

    let (input, sig) = nom_u32(data, "VHDX region table signature truncated")?;
    let magic = 0x69676572;
    if sig != magic {
        return Err(AccessorError::volume(format!(
            "VHDX region table signature wrong. Wanted {magic:#0x}, got: {sig:#0x}",
        )));
    }

    let (input, checksum) = nom_u32(input, "VHDX region table checksum truncated")?;
    let block = &data[..region_size];
    let value = vhdx_crc32c(block, region_size);

    if checksum != value {
        return Err(AccessorError::volume(format!(
            "VHDX region table checksum incorrect. Wanted {value:#0x}, got: {checksum:#0x}",
        )));
    }

    let (input, count) = nom_u32(input, "VHDX region table entry count truncated")?;

    let max_entries = 2047;
    if count > max_entries {
        return Err(AccessorError::volume(format!(
            "VHDX region table entry count {count} is above {max_entries}"
        )));
    }

    let (_, reserved) = nom_u32(input, "VHDX region table reserved truncated")?;
    if reserved != 0 {
        return Err(AccessorError::volume(format!(
            "VHDX region table reserved field is {reserved}"
        )));
    }

    let mut entries = Vec::with_capacity(count as usize);
    for index in 0..count {
        let start = 16 + usize::try_from(index).unwrap_or(usize::MAX) * 32;
        let entry = block.get(start..start + 32).ok_or_else(|| {
            AccessorError::volume(format!("VHDX region entry {index} is truncated"))
        })?;

        entries.push(parse_entry(index, entry, &entries)?);
    }

    check_overlaps(&entries)?;

    Ok(entries)
}

/// Return the active region table for the VHDX file
pub(super) fn active_regions(first: &[u8], second: &[u8]) -> AccessorResult<VhdxRegionTable> {
    let entries = match (parse_region_table(first), parse_region_table(second)) {
        (Ok(first_table), Ok(second_table)) if first_table != second_table => {
            return Err(AccessorError::volume(
                "VHDX region table copies do not match",
            ));
        }
        (Ok(first_table), Ok(_) | Err(_)) => first_table,
        (Err(_), Ok(second_table)) => second_table,
        (Err(first_err), Err(second_err)) => {
            return Err(AccessorError::volume(format!(
                "Both VHDX region table copies are invalid: {first_err:?}; {second_err:?}"
            )));
        }
    };

    let bat_guid = Uuid::from_u128(0x2dc27766f62342009d64115e9bfd4a08);
    let bat = required_region(&entries, bat_guid, "BAT")?;

    let meta_guid = Uuid::from_u128(0x8b7ca20647904b9ab8fe575f050f886e);
    let metadata = required_region(&entries, meta_guid, "METADATA")?;

    for entry in &entries {
        if entry.guid != bat_guid && entry.guid != meta_guid && entry.required {
            return Err(AccessorError::volume(format!(
                "VHDX region {} is required and is not supported",
                entry.guid
            )));
        }
    }

    Ok(VhdxRegionTable { bat, metadata })
}

/// Parse a region entry
fn parse_entry(index: u32, data: &[u8], seen: &[VhdxRegion]) -> AccessorResult<VhdxRegion> {
    let (input, guid) = nom_guid(data, "VHDX region GUID truncated")?;
    let (input, file_offset) = nom_u64(input, "VHDX region offset truncated")?;
    let (input, length) = nom_u32(input, "VHDX region length truncated")?;
    let (_, flags) = nom_u32(input, "VHDX region flags truncated")?;

    if flags & !1 != 0 {
        return Err(AccessorError::volume(format!(
            "VHDX region entry {index} has unknown required bits {flags:#x}"
        )));
    }

    let bytes_aligned = 1024 * 1024;
    if file_offset < bytes_aligned || !file_offset.is_multiple_of(bytes_aligned) {
        return Err(AccessorError::volume(format!(
            "VHDX region entry {index} offset {file_offset} is not 1 MiB aligned"
        )));
    }

    if length == 0 || !(length as u64).is_multiple_of(bytes_aligned) {
        return Err(AccessorError::volume(format!(
            "VHDX region entry {index} length {length} is not a non-zero multiple of 1 MiB"
        )));
    }

    if file_offset.checked_add(length as u64).is_none() {
        return Err(AccessorError::volume(format!(
            "VHDX region entry {index} range overflows"
        )));
    }

    if seen.iter().any(|region| region.guid == guid) {
        return Err(AccessorError::volume(format!(
            "VHDX region table contains a duplicate GUID {guid}"
        )));
    }

    Ok(VhdxRegion {
        guid,
        file_offset,
        length,
        required: flags & 1 != 0,
    })
}

/// Validate that the BAT and Metadata values do not overlap
fn check_overlaps(entries: &[VhdxRegion]) -> AccessorResult<()> {
    for (index, entry) in entries.iter().enumerate() {
        for other in &entries[index + 1..] {
            if ranges_overlap(
                entry.file_offset,
                entry.length as u64,
                other.file_offset,
                other.length as u64,
            ) {
                return Err(AccessorError::volume("VHDX region table entries overlap"));
            }
        }
    }

    Ok(())
}

/// Validate the region table contains provided region ID
fn required_region(entries: &[VhdxRegion], guid: Uuid, name: &str) -> AccessorResult<VhdxRegion> {
    entries
        .iter()
        .find(|entry| entry.guid == guid)
        .cloned()
        .ok_or_else(|| AccessorError::volume(format!("VHDX region table is missing the {name}")))
}

#[cfg(test)]
mod test {
    use super::{active_regions, parse_region_table};
    use crate::accessor::{
        disk::vhdx::header::vhdx_crc32c,
        error::{AccessorError, AccessorResult},
    };
    use std::{fs::read, path::PathBuf};
    use uuid::Uuid;

    fn put_u32(bytes: &mut [u8], offset: usize, value: u32) {
        bytes[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u64(bytes: &mut [u8], offset: usize, value: u64) {
        bytes[offset..offset + 8].copy_from_slice(&value.to_le_bytes());
    }

    fn put_guid(bytes: &mut [u8], offset: usize, guid: Uuid) {
        bytes[offset..offset + 16].copy_from_slice(&guid.to_bytes_le());
    }

    fn region_bytes(entries: &[(Uuid, u64, u32, u32)]) -> Vec<u8> {
        let mut bytes = vec![0_u8; 65536];
        put_u32(&mut bytes, 0, 0x69676572);
        put_u32(&mut bytes, 8, entries.len() as u32);

        for (index, (guid, file_offset, length, flags)) in entries.iter().enumerate() {
            let start = 16 + index * 32;
            put_guid(&mut bytes, start, *guid);
            put_u64(&mut bytes, start + 16, *file_offset);
            put_u32(&mut bytes, start + 24, *length);
            put_u32(&mut bytes, start + 28, *flags);
        }

        let checksum = vhdx_crc32c(&bytes, 65536);
        put_u32(&mut bytes, 4, checksum);
        bytes
    }

    fn standard_entries() -> Vec<(Uuid, u64, u32, u32)> {
        vec![
            (
                Uuid::from_u128(0x2dc27766f62342009d64115e9bfd4a08),
                0x200000,
                1048576 as u32,
                0,
            ),
            (
                Uuid::from_u128(0x8b7ca20647904b9ab8fe575f050f886e),
                0x300000,
                1048576 as u32,
                0,
            ),
        ]
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
        bytes[start..start + 65536].to_vec()
    }

    #[test]
    fn test_active_regions_parses_bat_and_metadata() {
        let bytes = region_bytes(&standard_entries());
        let table = active_regions(&bytes, &bytes).unwrap();

        assert_eq!(
            table.bat.guid,
            Uuid::from_u128(0x2dc27766f62342009d64115e9bfd4a08)
        );
        assert_eq!(table.bat.file_offset, 0x200000);
        assert_eq!(table.bat.length, 1048576 as u32);
        assert!(!table.bat.required);

        assert_eq!(
            table.metadata.guid,
            Uuid::from_u128(0x8b7ca20647904b9ab8fe575f050f886e)
        );
        assert_eq!(table.metadata.file_offset, 0x300000);
        assert_eq!(table.metadata.length, 1048576 as u32);
    }

    #[test]
    fn test_active_regions_requires_bat_and_metadata() {
        let metadata_only = region_bytes(&[(
            Uuid::from_u128(0x8b7ca20647904b9ab8fe575f050f886e),
            0x300000,
            1048576 as u32,
            0,
        )]);
        let reason = volume_reason(active_regions(&metadata_only, &metadata_only));
        assert!(reason.contains("missing the BAT"), "{reason}");

        let bat_only = region_bytes(&[(
            Uuid::from_u128(0x2dc27766f62342009d64115e9bfd4a08),
            0x200000,
            1048576 as u32,
            0,
        )]);
        let reason = volume_reason(active_regions(&bat_only, &bat_only));
        assert!(reason.contains("missing the METADATA"), "{reason}");
    }

    #[test]
    fn test_active_regions_unknown_entries() {
        let unknown = Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap();
        let mut entries = standard_entries();
        entries.push((unknown, 0x400000, 1048576 as u32, 0));
        let table = active_regions(&region_bytes(&entries), &region_bytes(&entries)).unwrap();
        assert_eq!(
            table.bat.guid,
            Uuid::from_u128(0x2dc27766f62342009d64115e9bfd4a08)
        );
        assert_eq!(
            table.metadata.guid,
            Uuid::from_u128(0x8b7ca20647904b9ab8fe575f050f886e)
        );

        entries[2].3 = 1;
        let reason = volume_reason(active_regions(
            &region_bytes(&entries),
            &region_bytes(&entries),
        ));
        assert!(reason.contains("not supported"), "{reason}");
    }

    #[test]
    fn test_active_regions_ignores_one_bad_copy() {
        let mut first = region_bytes(&standard_entries());
        first[100] ^= 0xff;
        let second = region_bytes(&standard_entries());
        let table = active_regions(&first, &second).unwrap();
        assert_eq!(table.bat.file_offset, 0x200000);
    }

    #[test]
    fn test_active_regions_rejects_mismatched_and_invalid_copies() {
        let first = region_bytes(&standard_entries());
        let second = region_bytes(&[
            (
                Uuid::from_u128(0x2dc27766f62342009d64115e9bfd4a08),
                0x400000,
                1048576 as u32,
                0,
            ),
            (
                Uuid::from_u128(0x8b7ca20647904b9ab8fe575f050f886e),
                0x500000,
                1048576 as u32,
                0,
            ),
        ]);
        let reason = volume_reason(active_regions(&first, &second));
        assert!(reason.contains("do not match"), "{reason}");

        let reason = volume_reason(active_regions(&[0_u8; 16], &[0_u8; 16]));
        assert!(reason.contains("Both VHDX region table copies"), "{reason}");
    }

    #[test]
    fn test_parse_region_table_rejects_bad_entries() {
        let mut unaligned = standard_entries();
        unaligned[0].1 = 0x200000 + 512;
        let reason = volume_reason(parse_region_table(&region_bytes(&unaligned)));
        assert!(reason.contains("not 1 MiB aligned"), "{reason}");

        let overlapping = vec![
            (
                Uuid::from_u128(0x2dc27766f62342009d64115e9bfd4a08),
                0x200000,
                (2 * 1048576) as u32,
                0,
            ),
            (
                Uuid::from_u128(0x8b7ca20647904b9ab8fe575f050f886e),
                0x300000,
                1048576 as u32,
                0,
            ),
        ];
        let reason = volume_reason(parse_region_table(&region_bytes(&overlapping)));
        assert!(reason.contains("overlap"), "{reason}");

        let duplicate = vec![
            (
                Uuid::from_u128(0x2dc27766f62342009d64115e9bfd4a08),
                0x200000,
                1048576 as u32,
                0,
            ),
            (
                Uuid::from_u128(0x2dc27766f62342009d64115e9bfd4a08),
                0x400000,
                1048576 as u32,
                0,
            ),
        ];
        let reason = volume_reason(parse_region_table(&region_bytes(&duplicate)));
        assert!(reason.contains("duplicate GUID"), "{reason}");

        let mut reserved = region_bytes(&standard_entries());
        put_u32(&mut reserved, 12, 1);
        let value = vhdx_crc32c(&reserved, 65536);
        put_u32(&mut reserved, 4, value);

        let reason = volume_reason(parse_region_table(&reserved));
        assert!(reason.contains("reserved"), "{reason}");

        let mut count = region_bytes(&standard_entries());
        put_u32(&mut count, 8, 2048);
        let value = vhdx_crc32c(&count, 65536);
        put_u32(&mut count, 4, value);

        let reason = volume_reason(parse_region_table(&count));
        assert!(reason.contains("2047"), "{reason}");
    }

    #[test]
    fn test_active_regions_fixture() {
        let first = fixture_copy(196608);
        let second = fixture_copy(262144);
        let table = active_regions(&first, &second).unwrap();

        assert_eq!(table.bat.file_offset, 0x200000);
        assert_eq!(table.bat.length, 0x100000);
        assert!(!table.bat.required);

        assert_eq!(table.metadata.file_offset, 0x300000);
        assert_eq!(table.metadata.length, 0x100000);
        assert!(!table.metadata.required);
    }
}
