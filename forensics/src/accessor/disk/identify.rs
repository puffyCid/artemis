use crate::{
    accessor::{
        disk::inspect::{DiskLayout, DiskPartition, read_at},
        error::{AccessorError, AccessorResult},
    },
    utils::nom_helper::nom_u16,
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
        identified.push(IdentifiedPartition {
            filesystem: identify_partition(reader, partition)?,
            partition: partition.clone(),
        })
    }

    Ok(identified)
}

/// Identify the filesystem for the partition
fn identify_partition<R: Read + Seek>(
    reader: &mut R,
    partition: &DiskPartition,
) -> AccessorResult<FilesystemKind> {
    if let Some(kind) = is_ntfs(reader, partition)? {
        return Ok(kind);
    }

    Ok(FilesystemKind::Unknown)
}

/// Check if the partition is using NTFS filesystem
fn is_ntfs<R: Read + Seek>(
    reader: &mut R,
    partition: &DiskPartition,
) -> AccessorResult<Option<FilesystemKind>> {
    let boot_size = 512;
    if partition.byte_length < boot_size {
        return Ok(None);
    }

    let sector = read_at(reader, partition.byte_offset, boot_size as usize)?;
    let sig = boot_signature(&sector)?;

    let boot_sig = 0xaa55;
    if sig != boot_sig {
        return Ok(None);
    }

    let kind = match sector.get(3..11) {
        Some(b"NTFS    ") => FilesystemKind::Ntfs,
        _ => return Ok(None),
    };

    Ok(Some(kind))
}

/// Return the boot signature for the partition
fn boot_signature(sector: &[u8]) -> AccessorResult<u16> {
    let footer = sector
        .get(510..512)
        .ok_or_else(|| AccessorError::volume("boot sector is shorter than 512 bytes"))?;

    let (_, sig) = nom_u16(footer, "Boot sector footer signature is truncated")?;

    Ok(sig)
}

#[cfg(test)]
mod tests {
    use crate::accessor::disk::{
        identify::{FilesystemKind, boot_signature, identify_disk, identify_partition, is_ntfs},
        inspect::{DiskLayout, DiskPartition, PartitionKind, PartitionTableKind},
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
    fn test_boot_signature() {
        let boot = boot_sector(b"testtest", &[]);
        let result = boot_signature(&boot).unwrap();
        assert_eq!(result, 0xaa55);
    }

    #[test]
    fn test_is_ntfs() {
        let sector = boot_sector(b"NTFS    ", &[]);
        let result = is_ntfs(&mut Cursor::new(sector), &gpt_partition(0, 512))
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
}
