use crate::{
    accessor::error::{AccessorError, AccessorResult},
    utils::nom_helper::{nom_guid, nom_take, nom_u16, nom_u32, nom_u64},
};
use uuid::Uuid;

/// File parameters item
const FILE_PARAMETERS: Uuid = Uuid::from_u128(0xcaa16737fa364d43b3b633f0aa44e76b);
/// Virtual disk size item
const VIRTUAL_DISK_SIZE: Uuid = Uuid::from_u128(0x2fa54224cd1b4876b2115dbed83bf4b8);
/// SCSI page 83 disk identifier item
const PAGE83_DATA: Uuid = Uuid::from_u128(0xbeca12abb2e6452393efc309e000c746);
/// Logical sector size item
const LOGICAL_SECTOR_SIZE: Uuid = Uuid::from_u128(0x8141bf1da96f4709ba47f233a8faab5f);
/// Physical sector size item
const PHYSICAL_SECTOR_SIZE: Uuid = Uuid::from_u128(0xcda348c7445d44719cc9e9885251c556);
/// Parent locator item. Used on differencing disks (Snapshots)
const PARENT_LOCATOR: Uuid = Uuid::from_u128(0xa8d35f2db30b454dabf7d3d84834ab0c);

/// Metadata has `IsUser` flag
const IS_USER: u32 = 0x1;
/// Metadata has `IsVirtual` flag
const IS_VIRTUAL_DISK: u32 = 0x2;
/// Metadata has `IsRequired` flag
const IS_REQUIRED: u32 = 0x4;
/// Supported VHDX flags
const ENTRY_FLAG_MASK: u32 = IS_USER | IS_VIRTUAL_DISK | IS_REQUIRED;

/// Parsed VHDX metadata
///
/// <https://github.com/libyal/libvhdi/blob/main/documentation/Virtual%20Hard%20Disk%20version%202%20(VHDX)%20image%20format.asciidoc#5-metadata-region>
#[derive(Debug, Clone, PartialEq)]
pub(super) struct VhdxMetadata {
    /// Block size in bytes
    ///
    /// Should be power of two (1MB through 256 MB)
    pub(super) block_size: u32,
    /// VHDX file is fixed type
    pub(super) leave_blocks_allocated: bool,
    /// Virtual disk size in bytes
    pub(super) virtual_disk_size: u64,
    /// SCSI page 83 disk ID
    ///
    /// <https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/managing_file_systems/assembly_overview-of-persistent-naming-attributes_managing-file-systems#device-identifiers_assembly_overview-of-persistent-naming-attributes>
    pub(super) disk_id: Uuid,
    /// Logical sector size in bytes
    ///
    /// Will be either 512 or 4096 bytes
    pub(super) logical_sector_size: u32,
    /// Physical sector size in bytes
    ///
    /// Will be either 512 or 4096 bytes
    pub(super) physical_sector_size: u32,
    /// Payload BAT entries between bitmap entries
    pub(super) chunk_ratio: u32,
}

/// Parsed Metadata entry value
///
/// Snapshots (parent locator) not supported yet
///
/// <https://github.com/libyal/libvhdi/blob/main/documentation/Virtual%20Hard%20Disk%20version%202%20(VHDX)%20image%20format.asciidoc#522-file-parameters-metadata-item>
struct MetadataEntry {
    /// Index for the metadata entry
    index: u16,
    /// Metadata entry ID
    /// Current 6 types are known
    ///
    /// <https://docs.redhat.com/en/documentation/red_hat_enterprise_linux/9/html/managing_file_systems/assembly_overview-of-persistent-naming-attributes_managing-file-systems#device-identifiers_assembly_overview-of-persistent-naming-attributes>
    guid: Uuid,
    /// Metadata entry offset
    offset: u32,
    /// Metadata entry size
    length: u32,
    /// Metadata entry flags
    flags: u32,
}

/// Parse VHDX metadata region
pub(super) fn parse_metadata(data: &[u8]) -> AccessorResult<VhdxMetadata> {
    let (input, sig) = nom_u64(data, "VHDX metadata sig truncated")?;
    let magic = 0x617461646174656d;
    if sig != magic {
        return Err(AccessorError::volume(format!(
            "VHDX metadata signature wrong. Wanted {magic:#0x}, got: {sig:#0x}",
        )));
    }

    let (input, reserved) = nom_u16(input, "VHDX metadata reserved truncated")?;
    if reserved != 0 {
        return Err(AccessorError::volume(format!(
            "VHDX metadata reserved field is {reserved}"
        )));
    }

    let (input, count) = nom_u16(input, "VHDX metadata entry count truncated")?;
    let max_entries = 247;
    if count > max_entries {
        return Err(AccessorError::volume(format!(
            "VHDX metadata entry count {count} is above {max_entries}"
        )));
    }

    let (_, _reserved) = nom_take(input, 20_u8, "VHDX metadata reserved truncated")?;

    let table_length = 32 + count as usize * 32;
    if data.len() < table_length {
        return Err(AccessorError::volume(format!(
            "VHDX metadata table is {} bytes. Wanted at least {table_length}",
            data.len()
        )));
    }

    let mut entries = Vec::with_capacity(count as usize);
    for index in 0..count {
        let start = 32 + index as usize * 32;
        let entry = parse_entry(index, &data[start..start + 32], &entries, data.len())?;

        entries.push(entry);
    }

    check_overlap(&entries)?;
    validate_flags(&entries)?;

    let file_parameters = required_item(&entries, FILE_PARAMETERS, "file parameters")?;
    let virtual_disk_size = required_item(&entries, VIRTUAL_DISK_SIZE, "virtual disk")?;
    let page83 = required_item(&entries, PAGE83_DATA, "page 83 data")?;
    let logical_sector = required_item(&entries, LOGICAL_SECTOR_SIZE, "logical sector size")?;
    let physical_sector = required_item(&entries, PHYSICAL_SECTOR_SIZE, "physical sector size")?;

    let (block_size, leave_blocks_allocated) = parse_file_parameters(data, file_parameters)?;
    let virtual_disk_size = parse_virtual_disk_size(data, virtual_disk_size)?;
    let disk_id = parse_disk_id(data, page83)?;
    let logical_sector_size = parse_sector_size(data, logical_sector, "logical")?;
    let physical_sector_size = parse_sector_size(data, physical_sector, "physical")?;

    let max_size = 64 * 1024 * 1024 * 1024 * 1024;
    if virtual_disk_size == 0 || virtual_disk_size > max_size {
        return Err(AccessorError::volume(format!(
            "VHDX virtual disk size {virtual_disk_size} is invalid"
        )));
    }

    if virtual_disk_size % logical_sector_size as u64 != 0 {
        return Err(AccessorError::volume(format!(
            "VHDX virtual disk size {virtual_disk_size} is not a multiple of logical sector size {logical_sector_size}"
        )));
    }

    let chunk_ratio = chunk_ratio(block_size, logical_sector_size)?;

    Ok(VhdxMetadata {
        block_size,
        leave_blocks_allocated,
        virtual_disk_size,
        disk_id,
        logical_sector_size,
        physical_sector_size,
        chunk_ratio,
    })
}

/// Parse each Metadata entry data
fn parse_entry(
    index: u16,
    data: &[u8],
    seen: &[MetadataEntry],
    region_len: usize,
) -> AccessorResult<MetadataEntry> {
    let (input, guid) = nom_guid(data, "VHDX metadata GUID truncated")?;
    let (input, offset) = nom_u32(input, "VHDX metadata offset truncated")?;
    let (input, length) = nom_u32(input, "VHDX metadata length truncated")?;
    let (input, flags) = nom_u32(input, "VHDX metadata flags truncated")?;
    let (_, reserved) = nom_u32(input, "VHDX metadata reserved truncated")?;

    if flags & !ENTRY_FLAG_MASK != 0 {
        return Err(AccessorError::volume(format!(
            "VHDX metadata entry {index} has unknown flag bits {flags:#x}"
        )));
    }

    if reserved != 0 {
        return Err(AccessorError::volume(format!(
            "VHDX metadata entry {index} reserved field is {reserved}"
        )));
    }

    if length == 0 {
        return Err(AccessorError::volume(format!(
            "VHDX metadata entry {index} length is zero"
        )));
    }

    let meta_table_size = 65536;
    if offset < meta_table_size {
        return Err(AccessorError::volume(format!(
            "VHDX metadata entry {index} offset {offset} is inside the metadata table"
        )));
    }

    let end = u64::from(offset)
        .checked_add(u64::from(length))
        .ok_or_else(|| {
            AccessorError::volume(format!("VHDX metadata entry {index} range overflows"))
        })?;

    if end > region_len as u64 {
        return Err(AccessorError::volume(format!(
            "VHDX metadata entry {index} extends past the metadata region"
        )));
    }

    if seen.iter().any(|entry| entry.guid == guid) {
        return Err(AccessorError::volume(format!(
            "VHDX metadata table contains a duplicate GUID {guid}"
        )));
    }

    Ok(MetadataEntry {
        index,
        guid,
        offset,
        length,
        flags,
    })
}

/// Validate each Metadata entry does not overlap with each other
fn check_overlap(entries: &[MetadataEntry]) -> AccessorResult<()> {
    for (index, entry) in entries.iter().enumerate() {
        let end = entry.offset + entry.length;

        for other in &entries[index + 1..] {
            let other_end = other.offset + other.length;
            if entry.offset < other_end && other.offset < end {
                return Err(AccessorError::volume("VHDX metadata entries overlap"));
            }
        }
    }

    Ok(())
}

/// Confirm the Metadata entry flags
fn validate_flags(entries: &[MetadataEntry]) -> AccessorResult<()> {
    for entry in entries {
        if entry.guid == PARENT_LOCATOR {
            return Err(AccessorError::volume(
                "VHDX differencing disks are not supported",
            ));
        }

        if entry.guid == FILE_PARAMETERS && entry.flags == IS_REQUIRED {
            continue;
        }

        let is_user = entry.flags & IS_USER != 0;
        let is_virtual = entry.flags & IS_VIRTUAL_DISK != 0;
        if is_user && is_virtual {
            return Err(AccessorError::volume(format!(
                "VHDX metadata entry {} flags {:#x} must set one of IsUser or IsVirtualDisk",
                entry.index, entry.flags
            )));
        }

        if is_required_virtual_item(entry.guid) {
            if !is_virtual || entry.flags & IS_REQUIRED == 0 {
                return Err(AccessorError::volume(format!(
                    "VHDX metadata item {} must be a required virtual disk item",
                    entry.guid
                )));
            }
            continue;
        }

        if entry.flags & IS_REQUIRED != 0 {
            return Err(AccessorError::volume(format!(
                "VHDX metadata item {} is required and is not supported",
                entry.guid
            )));
        }
    }

    Ok(())
}

/// Check if entry ID is a virtual item
fn is_required_virtual_item(guid: Uuid) -> bool {
    guid == FILE_PARAMETERS
        || guid == VIRTUAL_DISK_SIZE
        || guid == PAGE83_DATA
        || guid == LOGICAL_SECTOR_SIZE
        || guid == PHYSICAL_SECTOR_SIZE
}

/// Check if GUID is present in the parsed Metadata data
fn required_item<'a>(
    entries: &'a [MetadataEntry],
    guid: Uuid,
    name: &str,
) -> AccessorResult<&'a MetadataEntry> {
    entries
        .iter()
        .find(|entry| entry.guid == guid)
        .ok_or_else(|| AccessorError::volume(format!("VHDX metadata table is missing the {name}")))
}

/// Parse each Metadata entry item
fn item_bytes<'a>(data: &'a [u8], entry: &MetadataEntry) -> AccessorResult<&'a [u8]> {
    let start = usize::try_from(entry.offset).map_err(|err| {
        AccessorError::volume(format!(
            "VHDX metadata entry {} offset {} is outside the metadata region: {err:?}",
            entry.index, entry.offset
        ))
    })?;

    let length = usize::try_from(entry.length).map_err(|err| {
        AccessorError::volume(format!(
            "VHDX metadata entry {} length {} is outside the metadata region: {err:?}",
            entry.index, entry.length
        ))
    })?;

    let end = start.checked_add(length).ok_or_else(|| {
        AccessorError::volume(format!(
            "VHDX metadata entry {} range overflows",
            entry.index
        ))
    })?;

    data.get(start..end).ok_or_else(|| {
        AccessorError::volume(format!(
            "VHDX metadata entry {} extends past the metadata region",
            entry.index
        ))
    })
}

/// Parse the Metadata entry item
///
/// <https://github.com/libyal/libvhdi/blob/main/documentation/Virtual%20Hard%20Disk%20version%202%20(VHDX)%20image%20format.asciidoc#522-file-parameters-metadata-item>
fn parse_file_parameters(data: &[u8], entry: &MetadataEntry) -> AccessorResult<(u32, bool)> {
    if entry.length != 8 {
        return Err(AccessorError::volume(format!(
            "VHDX file parameters length is {}",
            entry.length
        )));
    }

    let bytes = item_bytes(data, entry)?;
    let (input, block_size) = nom_u32(bytes, "VHDX file parameters block size truncated")?;
    let (_, flags) = nom_u32(input, "VHDX file parameters flags truncated")?;

    let leave_blocks_allocated = 1;
    let has_parent = 2;
    if flags & !(leave_blocks_allocated | has_parent) != 0 {
        return Err(AccessorError::volume(format!(
            "VHDX file parameters flags {flags:#x} are reserved"
        )));
    }

    if flags & has_parent != 0 {
        return Err(AccessorError::volume(
            "VHDX differencing disks are not supported",
        ));
    }

    let min_block_size = 1024 * 1024;
    let max_block_size = 256 * 1024 * 1024;
    if block_size < min_block_size || block_size > max_block_size || !block_size.is_power_of_two() {
        return Err(AccessorError::volume(format!(
            "VHDX block size {block_size} is invalid"
        )));
    }

    Ok((block_size, flags & leave_blocks_allocated != 0))
}

/// Parse the Metadata entry item for virtual disk size
///
/// <https://github.com/libyal/libvhdi/blob/main/documentation/Virtual%20Hard%20Disk%20version%202%20(VHDX)%20image%20format.asciidoc#522-file-parameters-metadata-item>
fn parse_virtual_disk_size(data: &[u8], entry: &MetadataEntry) -> AccessorResult<u64> {
    if entry.length != 8 {
        return Err(AccessorError::volume(format!(
            "VHDX virtual disk size length is {}",
            entry.length
        )));
    }

    let bytes = item_bytes(data, entry)?;
    let (_, size) = nom_u64(bytes, "VHDX virtual disk size truncated")?;

    Ok(size)
}

/// Parse the Metadata entry disk ID
///
/// <https://github.com/libyal/libvhdi/blob/main/documentation/Virtual%20Hard%20Disk%20version%202%20(VHDX)%20image%20format.asciidoc#522-file-parameters-metadata-item>
fn parse_disk_id(data: &[u8], entry: &MetadataEntry) -> AccessorResult<Uuid> {
    if entry.length != 16 {
        return Err(AccessorError::volume(format!(
            "VHDX page 83 data length is {}",
            entry.length
        )));
    }

    let bytes = item_bytes(data, entry)?;
    let (_, disk_id) = nom_guid(bytes, "VHDX page 83 data truncated")?;

    Ok(disk_id)
}

/// Parse the logical Metadata entry item
///
/// <https://github.com/libyal/libvhdi/blob/main/documentation/Virtual%20Hard%20Disk%20version%202%20(VHDX)%20image%20format.asciidoc#522-file-parameters-metadata-item>
fn parse_sector_size(data: &[u8], entry: &MetadataEntry, name: &str) -> AccessorResult<u32> {
    if entry.length != 4 {
        return Err(AccessorError::volume(format!(
            "VHDX {name} sector size length is {}",
            entry.length
        )));
    }

    let bytes = item_bytes(data, entry)?;
    let (_, size) = nom_u32(bytes, "VHDX sector size truncated")?;
    if size != 512 && size != 4096 {
        return Err(AccessorError::volume(format!(
            "VHDX {name} sector size {size} is invalid"
        )));
    }

    Ok(size)
}

fn chunk_ratio(block_size: u32, logical_sector_size: u32) -> AccessorResult<u32> {
    let numerator = (1 << 23) * logical_sector_size as u64;
    if !block_size.is_power_of_two() || !numerator.is_multiple_of(block_size as u64) {
        return Err(AccessorError::volume(
            "VHDX block size and logical sector size do not produce a whole chunk ratio",
        ));
    }

    let ratio = numerator / block_size as u64;
    u32::try_from(ratio).map_err(|err| {
        AccessorError::volume(format!(
            "VHDX chunk ratio {ratio} does not fit in a u32: {err:?}"
        ))
    })
}

#[cfg(test)]
mod test {
    use super::parse_metadata;
    use crate::accessor::error::{AccessorError, AccessorResult};
    use std::{fs::read, path::PathBuf};
    use uuid::Uuid;

    struct Item {
        guid: Uuid,
        offset: u32,
        length: u32,
        flags: u32,
        payload: Vec<u8>,
    }

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

    fn u32_bytes(value: u32) -> Vec<u8> {
        value.to_le_bytes().to_vec()
    }

    fn u64_bytes(value: u64) -> Vec<u8> {
        value.to_le_bytes().to_vec()
    }

    fn file_parameters(block_size: u32, flags: u32) -> Vec<u8> {
        let mut bytes = Vec::with_capacity(8);
        bytes.extend(block_size.to_le_bytes());
        bytes.extend(flags.to_le_bytes());
        bytes
    }

    fn metadata_bytes(items: &[Item]) -> Vec<u8> {
        let mut size = 32 + items.len() * 32;
        for item in items {
            size = size.max(item.offset as usize + item.payload.len());
        }

        let mut bytes = vec![0_u8; size];
        put_u64(&mut bytes, 0, 0x6174_6164_6174_656d);
        put_u16(&mut bytes, 10, items.len() as u16);

        for item in items {
            if item.payload.is_empty() {
                continue;
            }
            let start = item.offset as usize;
            bytes[start..start + item.payload.len()].copy_from_slice(&item.payload);
        }
        for (index, item) in items.iter().enumerate() {
            let start = 32 + index * 32;
            put_guid(&mut bytes, start, item.guid);
            put_u32(&mut bytes, start + 16, item.offset);
            put_u32(&mut bytes, start + 20, item.length);
            put_u32(&mut bytes, start + 24, item.flags);
        }
        bytes
    }

    fn item(guid: Uuid, offset: u32, flags: u32, payload: Vec<u8>) -> Item {
        let length = payload.len() as u32;
        Item {
            guid,
            offset,
            length,
            flags,
            payload,
        }
    }

    fn standard_items() -> Vec<Item> {
        let disk_id = Uuid::parse_str("b08fa519-976f-2548-bb04-0866698648b6").unwrap();
        vec![
            item(
                Uuid::from_u128(0xcaa16737fa364d43b3b633f0aa44e76b),
                0x10000,
                0x6,
                file_parameters(0x800000, 0),
            ),
            item(
                Uuid::from_u128(0x2fa54224cd1b4876b2115dbed83bf4b8),
                0x10008,
                0x6,
                u64_bytes(8000000),
            ),
            item(
                Uuid::from_u128(0xbeca12abb2e6452393efc309e000c746),
                0x10010,
                0x6,
                disk_id.to_bytes_le().to_vec(),
            ),
            item(
                Uuid::from_u128(0x8141bf1da96f4709ba47f233a8faab5f),
                0x10020,
                0x6,
                u32_bytes(512),
            ),
            item(
                Uuid::from_u128(0xcda348c7445d44719cc9e9885251c556),
                0x10024,
                0x6,
                u32_bytes(512),
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

    fn fixture_metadata() -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/disks/vhdx/ntfs.vhdx");
        let bytes = read(path).unwrap();
        let start = 0x300000;
        bytes[start..start + 0x11000].to_vec()
    }

    #[test]
    fn test_parse_metadata_fixture() {
        let metadata = parse_metadata(&fixture_metadata()).unwrap();

        assert_eq!(metadata.block_size, 0x800000);
        assert!(!metadata.leave_blocks_allocated);
        assert_eq!(metadata.virtual_disk_size, 8_000_000);
        assert_eq!(
            metadata.disk_id,
            Uuid::parse_str("b08fa519-976f-2548-bb04-0866698648b6").unwrap()
        );

        assert_eq!(metadata.logical_sector_size, 512);
        assert_eq!(metadata.physical_sector_size, 512);
        assert_eq!(metadata.chunk_ratio, 512);
    }

    #[test]
    fn test_parse_metadata_dynamic_fixed_and_sector_sizes() {
        let metadata = parse_metadata(&metadata_bytes(&standard_items())).unwrap();
        assert_eq!(metadata.block_size, 0x800000);
        assert!(!metadata.leave_blocks_allocated);
        assert_eq!(metadata.chunk_ratio, 512);

        let mut fixed = standard_items();
        fixed[0].payload = file_parameters(0x800000, 1);
        let metadata = parse_metadata(&metadata_bytes(&fixed)).unwrap();
        assert!(metadata.leave_blocks_allocated);

        let mut sectors = standard_items();
        sectors[1].payload = u64_bytes(4096 * 100);
        sectors[3].payload = u32_bytes(4096);
        sectors[4].payload = u32_bytes(4096);

        let metadata = parse_metadata(&metadata_bytes(&sectors)).unwrap();
        assert_eq!(metadata.logical_sector_size, 4096);
        assert_eq!(metadata.physical_sector_size, 4096);
        assert_eq!(metadata.chunk_ratio, 4096);

        let mut largest = standard_items();
        largest[0].payload = file_parameters(256 * 1024 * 1024, 0);
        largest[1].payload = u64_bytes(64 * 1024 * 1024 * 1024 * 1024);

        let metadata = parse_metadata(&metadata_bytes(&largest)).unwrap();
        assert_eq!(metadata.block_size, 256 * 1024 * 1024);
        assert_eq!(metadata.virtual_disk_size, 64 * 1024 * 1024 * 1024 * 1024);
        assert_eq!(metadata.chunk_ratio, 16);
    }

    #[test]
    fn test_parse_metadata_qemu_file_parameters_and_optional_item() {
        let mut items = standard_items();
        items[0].flags = 0x4;
        items.push(item(
            Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap(),
            0x10030,
            0x1,
            u32_bytes(1),
        ));

        let metadata = parse_metadata(&metadata_bytes(&items)).unwrap();
        assert_eq!(metadata.virtual_disk_size, 8000000);

        items[1].flags = 0x4;
        let reason = volume_reason(parse_metadata(&metadata_bytes(&items)));
        assert!(reason.contains("required virtual disk item"), "{reason}");
    }

    #[test]
    fn test_parse_metadata_rejects_bad_table() {
        let reason = volume_reason(parse_metadata(&[0_u8; 32]));
        assert!(reason.contains("signature wrong"), "{reason}");

        let mut reserved = metadata_bytes(&standard_items());
        put_u16(&mut reserved, 8, 1);
        let reason = volume_reason(parse_metadata(&reserved));
        assert!(reason.contains("reserved field"), "{reason}");

        let mut count = metadata_bytes(&standard_items());
        put_u16(&mut count, 10, 2048);
        let reason = volume_reason(parse_metadata(&count));
        assert!(reason.contains("247"), "{reason}");

        let mut items = standard_items();
        items.push(item(
            Uuid::from_u128(0x2fa54224cd1b4876b2115dbed83bf4b8),
            0x20000,
            0x6,
            u64_bytes(8_000_000),
        ));

        let reason = volume_reason(parse_metadata(&metadata_bytes(&items)));
        assert!(reason.contains("duplicate GUID"), "{reason}");

        let mut items = standard_items();
        items[1].offset = 0x10004;
        let reason = volume_reason(parse_metadata(&metadata_bytes(&items)));
        assert!(reason.contains("overlap"), "{reason}");

        let mut items = standard_items();
        items[3].offset = 32;
        let reason = volume_reason(parse_metadata(&metadata_bytes(&items)));
        assert!(reason.contains("inside the metadata table"), "{reason}");

        let mut items = standard_items();
        items.remove(3);

        let reason = volume_reason(parse_metadata(&metadata_bytes(&items)));
        assert!(
            reason.contains("missing the logical sector size"),
            "{reason}"
        );
    }

    #[test]
    fn test_parse_metadata_rejects_bad_items() {
        let mut items = standard_items();
        items[0].payload = file_parameters(0x800000, 2);
        let reason = volume_reason(parse_metadata(&metadata_bytes(&items)));
        assert!(reason.contains("differencing"), "{reason}");

        let mut items = standard_items();
        items.push(item(
            Uuid::from_u128(0xa8d35f2db30b454dabf7d3d84834ab0c),
            0x20000,
            0x6,
            vec![0_u8; 20],
        ));

        let reason = volume_reason(parse_metadata(&metadata_bytes(&items)));
        assert!(reason.contains("differencing"), "{reason}");

        let mut items = standard_items();
        items.push(item(
            Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap(),
            0x20000,
            0x6,
            u32_bytes(1),
        ));

        let reason = volume_reason(parse_metadata(&metadata_bytes(&items)));
        assert!(reason.contains("not supported"), "{reason}");

        let mut items = standard_items();
        items[0].payload = file_parameters(3 * 1024 * 1024, 0);
        let reason = volume_reason(parse_metadata(&metadata_bytes(&items)));
        assert!(reason.contains("block size"), "{reason}");

        let mut items = standard_items();
        items[1].payload = u64_bytes(1000);
        let reason = volume_reason(parse_metadata(&metadata_bytes(&items)));
        assert!(reason.contains("not a multiple"), "{reason}");

        let mut items = standard_items();
        items[1].payload = u64_bytes((64 * 1024 * 1024 * 1024 * 1024) + 512);
        let reason = volume_reason(parse_metadata(&metadata_bytes(&items)));
        assert!(reason.contains("invalid"), "{reason}");

        let mut items = standard_items();
        items[3].payload = u32_bytes(1024);
        let reason = volume_reason(parse_metadata(&metadata_bytes(&items)));
        assert!(reason.contains("logical sector size 1024"), "{reason}");

        let mut items = standard_items();
        items[0].length = 4;
        let reason = volume_reason(parse_metadata(&metadata_bytes(&items)));
        assert!(reason.contains("file parameters length"), "{reason}");
    }
}
