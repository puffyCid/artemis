use crate::{
    accessor::{
        bootsector::mbr::PartitionType,
        config::AccessorConfig,
        disk::{
            inspect::{PartitionKind, PartitionTableKind},
            source::DiskSource,
        },
        error::AccessorError,
        location::loc::Location,
    },
    disks::{
        error::DiskResult,
        partition::{disk_format, table_name},
    },
    output::{manager::OutputManager, record::serialize_records_to_stream},
};
use serde::Serialize;

/// Boot-sector and partition-table metadata for a single partition
#[derive(Serialize)]
struct DiskBoot {
    /// Evidence path
    source: String,
    /// Partition ID name
    partition_id: String,
    /// Partition table type such as MBR
    table: String,
    /// MBR disk ID
    disk_id: Option<u32>,
    /// MBR active flag
    bootable: Option<bool>,
    /// MBR partition type
    partition_type: Option<String>,
    /// Raw MBR partition type byte
    partition_type_raw: Option<u8>,
    /// GPT disk GUID
    disk_guid: Option<String>,
    /// GPT partition GUID
    partition_guid: Option<String>,
    /// GPT partition type GUID
    type_guid: Option<String>,
    /// GPT partition name
    partition_name: Option<String>,
    /// GPT partition attributes
    attributes: Option<u64>,
}

/// Boot columns for a single partition
struct BootColumns {
    /// MBR disk ID
    disk_id: Option<u32>,
    /// MBR active flag
    bootable: Option<bool>,
    /// MBR partition type
    partition_type: Option<String>,
    /// Raw MBR partition type byte
    partition_type_raw: Option<u8>,
    /// GPT disk GUID
    disk_guid: Option<String>,
    /// GPT partition GUID
    partition_guid: Option<String>,
    /// GPT partition type GUID
    type_guid: Option<String>,
    /// GPT partition name
    partition_name: Option<String>,
    /// GPT partition attributes
    attributes: Option<u64>,
}

/// Get a little metadata on bootloader in the provided disk image
pub(crate) fn boot_info(source: &str, manager: &mut OutputManager) -> DiskResult<()> {
    let location = Location::parse_source(source)?;
    let format = disk_format(location.scheme)?;

    let image = location.source.ok_or_else(|| AccessorError::Location {
        input: source.to_string(),
        reason: String::from("Disk image paths must be absolute"),
    })?;

    let inspect =
        DiskSource::open(&AccessorConfig::default(), format, image.as_path())?.inspect()?;

    let table_kind = inspect.layout.table.clone();
    let table = table_name(&table_kind);

    let rows = inspect
        .partitions
        .into_iter()
        .map(|partition| {
            let columns = boot_columns(&table_kind, &partition.partition.kind);
            DiskBoot {
                source: source.to_string(),
                partition_id: partition.partition.id,
                table: table.to_string(),
                disk_id: columns.disk_id,
                bootable: columns.bootable,
                partition_type: columns.partition_type,
                partition_type_raw: columns.partition_type_raw,
                disk_guid: columns.disk_guid,
                partition_guid: columns.partition_guid,
                type_guid: columns.type_guid,
                partition_name: columns.partition_name,
                attributes: columns.attributes,
            }
        })
        .collect::<Vec<_>>();

    let mut records = serialize_records_to_stream(rows)?;
    manager.write_output("disk_boot", &mut records)?;

    Ok(())
}

/// Boot columns associated with the `PartitionKind`
fn boot_columns(table: &PartitionTableKind, kind: &PartitionKind) -> BootColumns {
    match kind {
        PartitionKind::Mbr {
            partition_type_raw,
            partition_type,
            bootable,
        } => BootColumns {
            disk_id: mbr_disk_id(table),
            bootable: Some(*bootable),
            partition_type: Some(partition_type_name(partition_type).to_string()),
            partition_type_raw: Some(*partition_type_raw),
            disk_guid: None,
            partition_guid: None,
            type_guid: None,
            partition_name: None,
            attributes: None,
        },
        PartitionKind::Gpt {
            type_guid,
            partition_guid,
            name,
            attributes,
        } => BootColumns {
            disk_id: None,
            bootable: None,
            partition_type: None,
            partition_type_raw: None,
            disk_guid: gpt_disk_guid(table),
            partition_guid: Some(partition_guid.to_string()),
            type_guid: Some(type_guid.to_string()),
            partition_name: Some(name.clone()),
            attributes: Some(*attributes),
        },
        PartitionKind::NtfsImage => BootColumns {
            disk_id: None,
            bootable: None,
            partition_type: None,
            partition_type_raw: None,
            disk_guid: None,
            partition_guid: None,
            type_guid: None,
            partition_name: None,
            attributes: None,
        },
    }
}

/// MBR disk ID when the table is MBR
fn mbr_disk_id(table: &PartitionTableKind) -> Option<u32> {
    match table {
        PartitionTableKind::Mbr { disk_id } => Some(*disk_id),
        PartitionTableKind::Gpt { .. } | PartitionTableKind::None => None,
    }
}
/// GPT disk GUID when the table is GPT
fn gpt_disk_guid(table: &PartitionTableKind) -> Option<String> {
    match table {
        PartitionTableKind::Gpt { disk_guid } => Some(disk_guid.to_string()),
        PartitionTableKind::Mbr { .. } | PartitionTableKind::None => None,
    }
}
/// MBR partition type as a stable output string
fn partition_type_name(kind: &PartitionType) -> &'static str {
    match kind {
        PartitionType::Ntfs => "ntfs",
        PartitionType::Linux => "linux",
        PartitionType::Unknown => "unknown",
        PartitionType::Fat16 => "fat16",
        PartitionType::Fat32 => "fat32",
        PartitionType::ExFat => "exfat",
        PartitionType::Protective => "protective",
        PartitionType::Extended => "extended",
        PartitionType::LinuxSwap => "linux_swap",
        PartitionType::LinuxLvm => "linux_lvm",
        PartitionType::Efi => "efi",
        PartitionType::None => "none",
    }
}

#[cfg(test)]
mod tests {
    use super::boot_info;
    use crate::{
        accessor::error::AccessorError,
        disks::error::DiskError,
        output::manager::OutputManager,
        structs::toml::{OutputConfig, OutputDestination, OutputFormat},
    };
    use std::{fs, path::PathBuf};
    use uuid::Uuid;

    fn test_image() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ntfs/test.raw");
        path
    }

    fn manager(name: &str) -> OutputManager {
        let output_dir = PathBuf::from("./tmp").join(name);
        let _ = fs::remove_dir_all(&output_dir);
        OutputManager::new(OutputConfig {
            name: name.to_string(),
            endpoint_id: String::from("test"),
            directory: PathBuf::from("./tmp"),
            destination: OutputDestination::Local,
            format: OutputFormat::Jsonl,
            ..Default::default()
        })
        .unwrap()
    }

    fn write_image(name: &str, bytes: &[u8]) -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tmp");
        path.push("disk_boot_images");
        fs::create_dir_all(&path).unwrap();

        path.push(name);
        fs::write(&path, bytes).unwrap();
        path
    }

    fn output_record(name: &str) -> serde_json::Value {
        let output_dir = PathBuf::from("./tmp").join(name);
        let mut jsonl = String::new();
        for entry in fs::read_dir(&output_dir).unwrap() {
            let file = entry.unwrap().path();
            let filename = file.file_name().unwrap().to_string_lossy();
            if filename.starts_with("disk_boot_") && filename.ends_with(".jsonl") {
                jsonl.push_str(&fs::read_to_string(&file).unwrap());
            }
        }

        assert_eq!(jsonl.lines().count(), 1);
        serde_json::from_str(jsonl.lines().next().unwrap()).unwrap()
    }

    fn assert_mbr_columns_empty(record: &serde_json::Value) {
        assert!(record["disk_id"].is_null());
        assert!(record["bootable"].is_null());
        assert!(record["partition_type"].is_null());
        assert!(record["partition_type_raw"].is_null());
    }

    fn assert_gpt_columns_empty(record: &serde_json::Value) {
        assert!(record["disk_guid"].is_null());
        assert!(record["partition_guid"].is_null());
        assert!(record["type_guid"].is_null());
        assert!(record["partition_name"].is_null());
        assert!(record["attributes"].is_null());
    }

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
    fn test_boot_info_logical_ntfs() {
        let image = test_image();
        let source = format!("raw:{}", image.display());
        let mut manager = manager("disk_boot_logical");
        boot_info(&source, &mut manager).unwrap();
        manager.finalize().unwrap();

        let record = output_record("disk_boot_logical");
        assert_eq!(record["source"], source);
        assert_eq!(record["partition_id"], "Partition0");
        assert_eq!(record["table"], "none");

        assert_mbr_columns_empty(&record);
        assert_gpt_columns_empty(&record);

        assert_eq!(record["collection_metadata"]["artifact_name"], "disk_boot");
    }

    #[test]
    fn test_boot_info_mbr() {
        let mut disk = mbr_sector(&[(0, 0x07, 2048, 1000)]);
        disk[440..444].copy_from_slice(&0x1234_5678u32.to_le_bytes());
        disk[446] = 0x80;

        let image = write_image("mbr.raw", &disk);
        let source = format!("raw:{}", image.display());
        let mut manager = manager("disk_boot_mbr");
        boot_info(&source, &mut manager).unwrap();

        manager.finalize().unwrap();
        let record = output_record("disk_boot_mbr");
        assert_eq!(record["source"], source);
        assert_eq!(record["partition_id"], "Partition0");
        assert_eq!(record["table"], "mbr");

        assert_eq!(record["disk_id"].as_u64(), Some(305419896));
        assert_eq!(record["bootable"], true);
        assert_eq!(record["partition_type"], "ntfs");
        assert_eq!(record["partition_type_raw"].as_u64(), Some(7));

        assert_gpt_columns_empty(&record);
    }

    #[test]
    fn test_boot_info_gpt() {
        let type_guid = Uuid::parse_str("ebd0a0a2-b9e5-4433-87c0-68b6b72699c7").unwrap();
        let disk_guid = Uuid::parse_str("11111111-2222-3333-4444-555555555555").unwrap();
        let partition_guid = Uuid::parse_str("aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee").unwrap();

        let used = gpt_entry(2048, 4095, "Windows", type_guid, partition_guid, 1);
        let image = write_image("gpt.raw", &gpt_disk(&used, disk_guid));
        let source = format!("raw:{}", image.display());
        let mut manager = manager("disk_boot_gpt");

        boot_info(&source, &mut manager).unwrap();
        manager.finalize().unwrap();
        let record = output_record("disk_boot_gpt");

        assert_eq!(record["source"], source);
        assert_eq!(record["partition_id"], "Partition1");
        assert_eq!(record["table"], "gpt");
        assert_eq!(record["disk_guid"], "11111111-2222-3333-4444-555555555555");

        assert_eq!(
            record["partition_guid"],
            "aaaaaaaa-bbbb-cccc-dddd-eeeeeeeeeeee"
        );
        assert_eq!(record["type_guid"], "ebd0a0a2-b9e5-4433-87c0-68b6b72699c7");
        assert_eq!(record["partition_name"], "Windows");
        assert_eq!(record["attributes"].as_u64(), Some(1));
        assert_mbr_columns_empty(&record);
    }

    #[test]
    fn test_boot_info_relative_source_is_error() {
        let mut manager = manager("disk_boot_relative");
        let err = boot_info("raw:image.raw", &mut manager).unwrap_err();
        assert!(matches!(
            err,
            DiskError::Source(AccessorError::Location { reason, .. }) if reason.contains("absolute")
        ));
    }
}
