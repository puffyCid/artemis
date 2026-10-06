use crate::accessor::{
    bootsector::{
        gpt::{GptEntry, parse_gpt_entries, parse_gpt_header},
        mbr::{MbrEntry, PartitionType, parse_mbr},
    },
    error::{AccessorError, AccessorResult},
};
use std::io::{Read, Seek, SeekFrom};
use uuid::Uuid;

/// Logical sector size for disk layout
const LOGIC_SECTOR_SIZE: u64 = 512;

/// The partition layout of a logical disk
#[derive(Debug, Clone, PartialEq)]
pub(super) struct DiskLayout {
    /// Used to determine the partition table
    pub(super) logical_sector_size: u64,
    /// Length of the image in bytes
    pub(super) image_size: u64,
    /// Source of the partition records
    pub(super) table: PartitionTableKind,
    /// Array of partitions
    pub(super) partitions: Vec<DiskPartition>,
}

/// Support partition record types
#[derive(Debug, Clone, PartialEq)]
pub(super) enum PartitionTableKind {
    /// Master Boot Record
    Mbr {
        /// MBR disk ID
        disk_id: u32,
    },
    /// GUID Partition Table
    Gpt {
        /// GPT disk GUID
        disk_guid: Uuid,
    },
    /// Filesystem with no partition table
    ///
    /// Example: Logical forensic image
    None,
}

/// Data associated with a single `DiskPartition`
#[derive(Debug, Clone, PartialEq)]
pub(super) struct DiskPartition {
    /// Readable partition ID
    ///
    /// Example: `Partition0`
    pub(super) id: String,
    /// Slot for partition in MBR or GPT array
    pub(super) slot: u32,
    /// First sector of the partition
    pub(super) start_lba: u64,
    /// Number of sectors in the partition
    pub(super) sector_count: u64,
    /// Byte offset from the start of the logical disk
    pub(super) byte_offset: u64,
    /// Byte length of the partition
    pub(super) byte_length: u64,
    /// Partition table metadata
    pub(super) kind: PartitionKind,
}

/// Partition table metadata
#[derive(Debug, Clone, PartialEq)]
pub(super) enum PartitionKind {
    /// MBR partition entry
    Mbr {
        /// Raw partition type number
        partition_type_raw: u8,
        /// Readable partition value
        partition_type: PartitionType,
        /// Is MBR partition bootable
        bootable: bool,
    },
    /// GPT partition entry
    Gpt {
        /// GPT partition value
        type_guid: Uuid,
        /// Unique GUID for the partition
        partition_guid: Uuid,
        /// GPT partition name
        name: String,
        /// GPT partition attributes
        attributes: u64,
    },
    /// Logical NTFS image
    NtfsImage,
}

/// Parse the disk partition and determine the layout
pub(super) fn inspect_disk<R: Read + Seek>(reader: &mut R) -> AccessorResult<DiskLayout> {
    let sector = read_at(reader, 0, LOGIC_SECTOR_SIZE as usize)?;
    let image_size = reader_length(reader)?;

    // We support logical NTFS images
    if is_logical_ntfs(&sector) {
        return Ok(DiskLayout {
            logical_sector_size: LOGIC_SECTOR_SIZE,
            table: PartitionTableKind::None,
            image_size,
            partitions: vec![logic_partition(image_size, PartitionKind::NtfsImage)],
        });
    }

    // Check for MBR first
    let mbr = parse_mbr(&sector)?;
    if mbr.entries.iter().any(MbrEntry::is_extended) {
        return Err(AccessorError::volume(
            "Extended MBR partitions are not supported",
        ));
    }

    if mbr.is_protective_gpt() {
        return inspect_gpt(reader, image_size);
    }

    Ok(DiskLayout {
        logical_sector_size: LOGIC_SECTOR_SIZE,
        image_size,
        table: PartitionTableKind::Mbr {
            disk_id: mbr.disk_id,
        },
        partitions: mbr_partitions(&mbr.entries)?,
    })
}

/// If we have GPT bootsector
///
/// Then parse GPT data
fn inspect_gpt<R: Read + Seek>(reader: &mut R, image_size: u64) -> AccessorResult<DiskLayout> {
    let header_sector = read_at(reader, LOGIC_SECTOR_SIZE, LOGIC_SECTOR_SIZE as usize)?;
    let header = parse_gpt_header(&header_sector)?;

    if header.current_lba != 1 {
        return Err(AccessorError::volume(format!(
            "Expected primary GPT header at LBA 1, found LBA {}",
            header.current_lba
        )));
    }

    let array_size = header.partition_array_size()?;
    let max_array = 1024 * 1024;
    if array_size > max_array {
        return Err(AccessorError::volume(format!(
            "GPT partition entry array is {array_size} bytes, above the {max_array} byte limit"
        )));
    }

    let array_offset = header.partition_array_offset(LOGIC_SECTOR_SIZE)?;
    let array_len = array_size as usize;

    let array_bytes = read_at(reader, array_offset, array_len)?;
    let entries = parse_gpt_entries(&array_bytes, &header)?;

    let mut partitions = Vec::with_capacity(entries.len());
    for entry in entries {
        partitions.push(gpt_partition(&entry)?);
    }

    Ok(DiskLayout {
        logical_sector_size: LOGIC_SECTOR_SIZE,
        table: PartitionTableKind::Gpt {
            disk_guid: header.disk_guid,
        },
        image_size,
        partitions,
    })
}

/// Loop through all MBR entries
fn mbr_partitions(entries: &[MbrEntry]) -> AccessorResult<Vec<DiskPartition>> {
    entries.iter().map(mbr_partition).collect()
}

/// Assemble `DiskPartition` from a MBR
fn mbr_partition(entry: &MbrEntry) -> AccessorResult<DiskPartition> {
    Ok(DiskPartition {
        id: partition_id(entry.slot as u32),
        slot: entry.slot as u32,
        start_lba: entry.start_lba as u64,
        sector_count: entry.sector_count as u64,
        byte_offset: entry.byte_offset(LOGIC_SECTOR_SIZE),
        byte_length: entry.byte_length(LOGIC_SECTOR_SIZE),
        kind: PartitionKind::Mbr {
            partition_type_raw: entry.partition_type_raw,
            partition_type: entry.partition_type.clone(),
            bootable: entry.is_bootable(),
        },
    })
}

/// Parse each identified GPT entry
fn gpt_partition(entry: &GptEntry) -> AccessorResult<DiskPartition> {
    let byte_length = entry.byte_length(LOGIC_SECTOR_SIZE)?;
    let byte_offset = entry
        .start_lba
        .checked_mul(LOGIC_SECTOR_SIZE)
        .ok_or_else(|| {
            AccessorError::volume(format!("GPT partition {} byte offset overflow", entry.slot))
        })?;

    Ok(DiskPartition {
        id: partition_id(entry.slot),
        slot: entry.slot,
        start_lba: entry.start_lba,
        sector_count: byte_length / LOGIC_SECTOR_SIZE,
        byte_offset,
        byte_length,
        kind: PartitionKind::Gpt {
            type_guid: entry.partition_type_guid,
            name: entry.partition_name.clone(),
            attributes: entry.attributes,
            partition_guid: entry.partition_guid,
        },
    })
}

/// If we have a logical disk
///
/// Assemble a `DiskPartition` from logical disk
fn logic_partition(length: u64, kind: PartitionKind) -> DiskPartition {
    DiskPartition {
        id: partition_id(0),
        slot: 0,
        start_lba: 0,
        sector_count: length / LOGIC_SECTOR_SIZE,
        byte_offset: 0,
        byte_length: length,
        kind,
    }
}

/// Check for logical NTFS disk
pub(super) fn is_logical_ntfs(sector: &[u8]) -> bool {
    sector.len() >= 512
        && &sector[3..11] == b"NTFS    "
        && u16::from_le_bytes([sector[510], sector[511]]) == 0xaa55
}

/// Create human readable partition name
fn partition_id(slot: u32) -> String {
    format!("Partition{slot}")
}

/// Return length of the disk
fn reader_length<R: Read + Seek>(reader: &mut R) -> AccessorResult<u64> {
    reader
        .seek(SeekFrom::End(0))
        .map_err(|err| AccessorError::volume(format!("Failed to determine disk length: {err}")))
}

/// Read bytes at provided offset
pub(super) fn read_at<R: Read + Seek>(
    reader: &mut R,
    offset: u64,
    len: usize,
) -> AccessorResult<Vec<u8>> {
    reader
        .seek(SeekFrom::Start(offset))
        .map_err(|err| AccessorError::volume(format!("Failed to seek to byte {offset}: {err}")))?;

    let mut buf = vec![0; len];
    reader.read_exact(&mut buf).map_err(|err| {
        AccessorError::volume(format!(
            "Failed to read {len} bytes at byte {offset}: {err}"
        ))
    })?;

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use crate::accessor::{
        bootsector::mbr::PartitionType,
        disk::inspect::{PartitionKind, PartitionTableKind, inspect_disk},
        error::AccessorError,
    };
    use std::io::Cursor;
    use uuid::Uuid;

    fn mbr_sector(entries: &[(u8, u8, u32, u32)]) -> Vec<u8> {
        let mut sector = vec![0u8; 512];
        sector[510] = 0x55;
        sector[511] = 0xAA;

        for &(slot, partition_type, start_lba, sector_count) in entries {
            let offset = 446 + usize::from(slot) * 16;
            sector[offset + 4] = partition_type;
            sector[offset + 8..offset + 12].copy_from_slice(&start_lba.to_le_bytes());
            sector[offset + 12..offset + 16].copy_from_slice(&sector_count.to_le_bytes());
        }

        sector
    }

    fn gpt_disk(second_entry: &[u8], disk_guid: Uuid) -> Vec<u8> {
        let mut disk = mbr_sector(&[(0, 0xEE, 1, 100)]);
        disk.extend(gpt_header(2, 2, disk_guid));
        disk.extend(vec![0u8; 128]);
        disk.extend_from_slice(second_entry);
        disk
    }

    fn gpt_header(entry_lba: u64, entry_count: u32, disk_guid: Uuid) -> Vec<u8> {
        let mut sector = vec![0u8; 512];
        sector[..8].copy_from_slice(b"EFI PART");
        sector[8..12].copy_from_slice(&0x0001_0000u32.to_le_bytes());
        sector[12..16].copy_from_slice(&92u32.to_le_bytes());
        sector[24..32].copy_from_slice(&1u64.to_le_bytes());

        sector[32..40].copy_from_slice(&9u64.to_le_bytes());
        sector[40..48].copy_from_slice(&3u64.to_le_bytes());
        sector[48..56].copy_from_slice(&8u64.to_le_bytes());
        sector[56..72].copy_from_slice(&disk_guid.to_bytes_le());
        sector[72..80].copy_from_slice(&entry_lba.to_le_bytes());
        sector[80..84].copy_from_slice(&entry_count.to_le_bytes());
        sector[84..88].copy_from_slice(&128u32.to_le_bytes());
        sector
    }

    fn gpt_entry(
        start_lba: u64,
        end_lba: u64,
        name: &str,
        type_guid: Uuid,
        partition_guid: Uuid,
        attributes: u64,
    ) -> Vec<u8> {
        let mut entry = vec![0u8; 128];
        entry[..16].copy_from_slice(&type_guid.to_bytes_le());
        entry[16..32].copy_from_slice(&partition_guid.to_bytes_le());
        entry[32..40].copy_from_slice(&start_lba.to_le_bytes());
        entry[40..48].copy_from_slice(&end_lba.to_le_bytes());
        entry[48..56].copy_from_slice(&attributes.to_le_bytes());

        for (index, unit) in name.encode_utf16().enumerate() {
            let offset = 56 + index * 2;
            entry[offset..offset + 2].copy_from_slice(&unit.to_le_bytes());
        }
        entry
    }

    #[test]
    fn test_inspect_mbr_partition() {
        let mut disk = mbr_sector(&[(0, 0x07, 2048, 1000)]);
        disk[440..444].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        disk[446] = 0x80;

        let layout = inspect_disk(&mut Cursor::new(disk)).unwrap();
        assert_eq!(layout.logical_sector_size, 512);
        assert_eq!(layout.table, PartitionTableKind::Mbr { disk_id: 305419896 });
        assert_eq!(layout.partitions.len(), 1);

        let partition = &layout.partitions[0];

        assert_eq!(partition.id, "Partition0");
        assert_eq!(partition.slot, 0);
        assert_eq!(partition.start_lba, 2048);

        assert_eq!(partition.sector_count, 1000);
        assert_eq!(partition.byte_offset, 1048576);
        assert_eq!(partition.byte_length, 512000);
        assert!(matches!(
            partition.kind,
            PartitionKind::Mbr {
                partition_type_raw: 0x07,
                partition_type: PartitionType::Ntfs,
                bootable: true,
            }
        ));

        assert_eq!(layout.image_size, 512);
    }

    #[test]
    fn test_inspect_mbr_preserves_table_slot() {
        let disk = mbr_sector(&[(0, 0x83, 2048, 100), (2, 0x07, 4096, 200)]);
        let layout = inspect_disk(&mut Cursor::new(disk)).unwrap();

        assert_eq!(layout.partitions.len(), 2);
        assert_eq!(layout.partitions[0].id, "Partition0");
        assert_eq!(layout.partitions[1].id, "Partition2");
        assert_eq!(layout.partitions[1].byte_offset, 2_097_152);
        assert_eq!(layout.partitions[1].byte_length, 102_400);
    }

    #[test]
    fn test_inspect_extended_mbr_is_rejected() {
        let disk = mbr_sector(&[(0, 0x0F, 100, 50)]);
        let error = inspect_disk(&mut Cursor::new(disk)).unwrap_err();
        assert!(matches!(
            error,
            AccessorError::Volume { reason } if reason.contains("Extended MBR")
        ));
    }

    #[test]
    fn test_inspect_logic_ntfs() {
        let mut disk = vec![0u8; 1024];
        disk[3..11].copy_from_slice(b"NTFS    ");
        disk[510] = 0x55;
        disk[511] = 0xAA;
        let layout = inspect_disk(&mut Cursor::new(disk)).unwrap();

        assert_eq!(layout.table, PartitionTableKind::None);
        assert_eq!(layout.partitions.len(), 1);
        assert_eq!(layout.partitions[0].id, "Partition0");

        assert_eq!(layout.partitions[0].byte_offset, 0);
        assert_eq!(layout.partitions[0].byte_length, 1024);
        assert!(matches!(
            layout.partitions[0].kind,
            PartitionKind::NtfsImage
        ));
    }

    #[test]
    fn test_inspect_gpt_preserves_array_slot() {
        let type_guid = Uuid::parse_str("ebd0a0a2-b9e5-4433-87c0-68b6b72699c7").unwrap();
        let disk_guid = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        let partition_guid = Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap();
        let used = gpt_entry(2048, 4095, "Windows", type_guid, partition_guid, 1);
        let disk = gpt_disk(&used, disk_guid);
        let layout = inspect_disk(&mut Cursor::new(disk)).unwrap();

        assert_eq!(layout.table, PartitionTableKind::Gpt { disk_guid });
        assert_eq!(layout.partitions.len(), 1);

        let partition = &layout.partitions[0];

        assert_eq!(partition.id, "Partition1");
        assert_eq!(partition.slot, 1);
        assert_eq!(partition.start_lba, 2048);

        assert_eq!(partition.sector_count, 2048);
        assert_eq!(partition.byte_offset, 1048576);
        assert_eq!(partition.byte_length, 1048576);

        match &partition.kind {
            PartitionKind::Gpt {
                type_guid: guid,
                name,
                attributes,
                partition_guid: unique,
            } => {
                assert_eq!(*guid, type_guid);
                assert_eq!(name, "Windows");
                assert_eq!(*attributes, 1);
                assert_eq!(*unique, partition_guid);
            }
            _ => panic!("expected a GPT partition"),
        }
    }

    #[test]
    fn test_inspect_missing_boot_signature() {
        let error = inspect_disk(&mut Cursor::new(vec![0u8; 512])).unwrap_err();
        assert!(matches!(
            error,
            AccessorError::Volume { reason } if reason.contains("Invalid MBR sig")
        ));
    }
}
