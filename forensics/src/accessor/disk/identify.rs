use tracing::warn;

use crate::accessor::{
    disk::inspect::{DiskLayout, DiskPartition, is_logical_ntfs, read_at},
    error::AccessorResult,
};
use std::io::{Read, Seek};

/// Filesystem found at partition offset
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum FilesystemKind {
    /// NTFS volume
    Ntfs,
    /// Ext4 volume
    Ext4,
    /// Bitlocker volume
    Bitlocker,
    /// Unknown filesystem found
    Unknown,
}

/// Partition with identified filesystem
#[derive(Debug, Clone, PartialEq)]
pub(super) struct IdentifiedPartition {
    /// Partition from inspected disk image
    pub(super) partition: DiskPartition,
    /// Filesystem identified
    pub(super) filesystem: FilesystemKind,
}

/// Determine the filesystem type for each partition on the disk
pub(super) fn identify_disk<R: Read + Seek>(
    reader: &mut R,
    layout: &DiskLayout,
) -> AccessorResult<Vec<IdentifiedPartition>> {
    let mut identified = Vec::with_capacity(layout.partitions.len());

    for partition in &layout.partitions {
        let filesystem = match identify_partition(reader, partition) {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    "Unknown partition for '{}'. Kind: {:?}. Length: {}: {err:?}",
                    partition.id, partition.kind, partition.byte_length
                );
                FilesystemKind::Unknown
            }
        };
        identified.push(IdentifiedPartition {
            filesystem,
            partition: partition.clone(),
        });
    }

    Ok(identified)
}

/// Identify the filesystem for the partition
fn identify_partition<R: Read + Seek>(
    reader: &mut R,
    partition: &DiskPartition,
) -> AccessorResult<FilesystemKind> {
    if let Some(kind) = check_filesystem(reader, partition)? {
        return Ok(kind);
    }

    Ok(FilesystemKind::Unknown)
}

/// Check for partition filesystem
fn check_filesystem<R: Read + Seek>(
    reader: &mut R,
    partition: &DiskPartition,
) -> AccessorResult<Option<FilesystemKind>> {
    let boot_size = 512;
    if partition.byte_length < boot_size {
        return Ok(None);
    }

    let sector = read_at(reader, partition.byte_offset, boot_size as usize)?;

    let kind = if is_logical_ntfs(&sector) {
        FilesystemKind::Ntfs
    } else {
        FilesystemKind::Unknown
    };

    Ok(Some(kind))
}

#[cfg(test)]
mod tests {
    use crate::accessor::disk::{
        identify::{FilesystemKind, check_filesystem, identify_disk, identify_partition},
        inspect::{DiskLayout, DiskPartition, PartitionKind, PartitionTableKind, inspect_disk},
    };
    use std::io::Cursor;

    fn boot_sector(oem: &[u8], extra: &[(usize, &[u8])]) -> Vec<u8> {
        let mut sector = vec![0u8; 512];
        sector[3..11].copy_from_slice(oem);

        for &(offset, value) in extra {
            sector[offset..offset + value.len()].copy_from_slice(value);
        }

        sector[510] = 0x55;
        sector[511] = 0xAA;
        sector
    }

    fn gpt_partition(byte_offset: u64, byte_length: u64) -> DiskPartition {
        DiskPartition {
            id: String::from("Partition0"),
            slot: 0,
            start_lba: byte_offset / 512,
            sector_count: byte_length / 512,
            byte_offset,
            byte_length,
            kind: PartitionKind::Gpt {
                type_guid: uuid::Uuid::nil(),
                name: String::new(),
            },
        }
    }

    fn write_mbr_entry(
        disk: &mut [u8],
        slot: u8,
        partition_type: u8,
        start_lba: u32,
        sector_count: u32,
    ) {
        let offset = 446 + usize::from(slot) * 16;
        disk[offset + 4] = partition_type;

        disk[offset + 8..offset + 12].copy_from_slice(&start_lba.to_le_bytes());
        disk[offset + 12..offset + 16].copy_from_slice(&sector_count.to_le_bytes());
    }

    #[test]
    fn test_identify_partition() {
        let sector = boot_sector(b"NTFS    ", &[]);
        let kind = identify_partition(&mut Cursor::new(sector), &gpt_partition(0, 512)).unwrap();

        assert_eq!(kind, FilesystemKind::Ntfs);
    }

    #[test]
    fn test_identify_partition_unknown() {
        let kind =
            identify_partition(&mut Cursor::new(vec![0u8; 512]), &gpt_partition(0, 512)).unwrap();

        assert_eq!(kind, FilesystemKind::Unknown);
    }

    #[test]
    fn test_check_filesystem() {
        let sector = boot_sector(b"NTFS    ", &[]);
        let result = check_filesystem(&mut Cursor::new(sector), &gpt_partition(0, 512))
            .unwrap()
            .unwrap();

        assert_eq!(result, FilesystemKind::Ntfs);
    }

    #[test]
    fn test_identify_disk() {
        let sector = boot_sector(b"NTFS    ", &[]);
        let partitions = vec![gpt_partition(0, 512)];
        let layout = DiskLayout {
            logical_sector_size: 0,
            table: PartitionTableKind::Gpt,
            partitions,
        };

        let result = identify_disk(&mut Cursor::new(sector), &layout).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0].filesystem, FilesystemKind::Ntfs);
    }

    #[test]
    fn test_identify_ntfs_at_partition_offset() {
        let mut disk = vec![0u8; 2560];
        disk[510] = 0x55;
        disk[511] = 0xAA;
        write_mbr_entry(&mut disk, 0, 0x07, 2, 1);
        write_mbr_entry(&mut disk, 2, 0x83, 4, 1);

        let ntfs = boot_sector(b"NTFS    ", &[]);
        let offset = 4 * 512;

        disk[offset..offset + ntfs.len()].copy_from_slice(&ntfs);
        let layout = inspect_disk(&mut Cursor::new(disk.clone())).unwrap();
        let found = identify_disk(&mut Cursor::new(disk), &layout).unwrap();

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].partition.id, "Partition0");
        assert_eq!(found[0].filesystem, FilesystemKind::Unknown);
        assert_eq!(found[0].partition.byte_offset, 1024);

        assert_eq!(found[1].partition.id, "Partition2");
        assert_eq!(found[1].filesystem, FilesystemKind::Ntfs);
        assert_eq!(found[1].partition.byte_offset, 2048);
        assert_eq!(found[1].partition.byte_length, 512);
    }

    #[test]
    fn test_identify_truncated_boot_sector_is_unknown() {
        let mut past_end = gpt_partition(1024, 512);
        past_end.id = String::from("Partition1");
        past_end.slot = 1;
        let layout = DiskLayout {
            logical_sector_size: 512,
            table: PartitionTableKind::Mbr,
            partitions: vec![gpt_partition(0, 512), past_end],
        };

        let found =
            identify_disk(&mut Cursor::new(boot_sector(b"NTFS    ", &[])), &layout).unwrap();

        assert_eq!(found.len(), 2);
        assert_eq!(found[0].filesystem, FilesystemKind::Ntfs);
        assert_eq!(found[1].filesystem, FilesystemKind::Unknown);
    }

    #[test]
    fn test_identify_short_partition_is_unknown() {
        let kind =
            identify_partition(&mut Cursor::new(vec![0u8; 512]), &gpt_partition(0, 100)).unwrap();
        assert_eq!(kind, FilesystemKind::Unknown);
    }
}
