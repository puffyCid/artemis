use crate::{
    accessor::{
        config::AccessorConfig,
        disk::{
            format::DiskFormat, identify::FilesystemKind, inspect::PartitionTableKind,
            source::DiskSource,
        },
        error::AccessorError,
        location::{loc::Location, scheme::Scheme},
    },
    disks::error::DiskResult,
    output::{manager::OutputManager, record::serialize_records_to_stream},
};
use serde::Serialize;

/// Metadata on a logical disk partition from an disk image
#[derive(Serialize)]
struct PartitionInfo {
    /// Evidence path
    source: String,
    /// Disk image format
    format: String,
    /// Logical sector size in bytes
    sector_size: u64,
    /// Partition table type such as MBR
    table: String,
    /// Number of partitions on the image
    partition_count: u64,
    /// Size of the image in bytes
    image_size: u64,
    /// Partition ID name
    partition_id: String,
    /// ID of the partition
    slot: u32,
    /// First sector of the partition
    start_lba: u64,
    /// Partition length in sectors
    sector_count: u64,
    /// Offset to the partition on the disk image
    byte_offset: u64,
    /// Partition length in bytes
    byte_length: u64,
    /// Detected filesystem
    filesystem: String,
}

/// Get some metadata on partitions in the provided disk image
pub(crate) fn partition_info(source: &str, manager: &mut OutputManager) -> DiskResult<()> {
    let location = Location::parse_source(source)?;
    let format = disk_format(location.scheme)?;

    let image = location.source.ok_or_else(|| AccessorError::Location {
        input: source.to_string(),
        reason: String::from("Disk image paths must be absolute"),
    })?;

    let inspect =
        DiskSource::open(&AccessorConfig::default(), format, image.as_path())?.inspect()?;

    let partition_count = inspect.partitions.len() as u64;
    let boot = boot_type(&inspect.layout.table);
    let format = inspect.format.as_str().to_string();

    let rows = inspect
        .partitions
        .into_iter()
        .map(|partition| PartitionInfo {
            source: source.to_string(),
            format: format.clone(),
            sector_size: inspect.layout.logical_sector_size,
            table: boot.to_string(),
            partition_count,
            image_size: inspect.layout.image_size,
            partition_id: partition.partition.id,
            slot: partition.partition.slot,
            start_lba: partition.partition.start_lba,
            sector_count: partition.partition.sector_count,
            byte_offset: partition.partition.byte_offset,
            byte_length: partition.partition.byte_length,
            filesystem: filesystem_name(partition.filesystem).to_string(),
        })
        .collect::<Vec<_>>();

    let mut records = serialize_records_to_stream(rows)?;
    manager.write_artifact("disk_info", &"", &mut records)?;

    Ok(())
}

/// Partition table kind as a stable output string
pub(super) fn boot_type(table: &PartitionTableKind) -> &'static str {
    match table {
        PartitionTableKind::Mbr { .. } => "mbr",
        PartitionTableKind::Gpt { .. } => "gpt",
        PartitionTableKind::None => "none",
    }
}

/// Disk container format for a source scheme
pub(super) fn disk_format(scheme: Scheme) -> Result<DiskFormat, AccessorError> {
    match scheme {
        Scheme::Raw => Ok(DiskFormat::Raw),
        Scheme::Host | Scheme::Ntfs | Scheme::Zip => Err(AccessorError::UnsupportedScheme {
            scheme: scheme.as_str().to_string(),
        }),
    }
}

/// Filesystem kind as a stable output string
fn filesystem_name(kind: FilesystemKind) -> &'static str {
    match kind {
        FilesystemKind::Ntfs => "ntfs",
        FilesystemKind::Ext4 => "ext4",
        FilesystemKind::Bitlocker => "bitlocker",
        FilesystemKind::Unknown => "unknown",
    }
}

#[cfg(test)]
mod tests {
    use super::partition_info;
    use crate::{
        accessor::error::AccessorError,
        disks::error::DiskError,
        output::manager::OutputManager,
        structs::toml::{OutputConfig, OutputDestination, OutputFormat},
    };
    use std::{fs, path::PathBuf};

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

    #[test]
    fn test_partition_info_logical_ntfs() {
        let image = test_image();
        let source = format!("raw:{}", image.display());
        let mut manager = manager("disk_info_logical");
        partition_info(&source, &mut manager).unwrap();
        manager.finalize().unwrap();

        let output_dir = PathBuf::from("./tmp/disk_info_logical");
        let mut jsonl = String::new();

        for entry in fs::read_dir(&output_dir).unwrap() {
            let file = entry.unwrap().path();
            let filename = file.file_name().unwrap().to_string_lossy();
            if filename.starts_with("disk_info_") && filename.ends_with(".jsonl") {
                jsonl.push_str(&fs::read_to_string(&file).unwrap());
            }
        }

        let record: serde_json::Value =
            serde_json::from_str(jsonl.lines().next().unwrap()).unwrap();
        let file_size = fs::metadata(&image).unwrap().len();

        assert_eq!(record["source"], source);
        assert_eq!(record["format"], "raw");
        assert_eq!(record["sector_size"], 512);
        assert_eq!(record["boot_type"], "none");
        assert_eq!(record["partition_count"], 1);

        assert_eq!(record["image_size"].as_u64(), Some(file_size));
        assert_eq!(record["partition_id"], "Partition0");
        assert_eq!(record["byte_offset"], 0);
        assert_eq!(record["filesystem"], "ntfs");
        assert_eq!(record["collection_metadata"]["artifact_name"], "disk_info");
    }

    #[test]
    fn test_partition_info_relative_source_is_error() {
        let mut manager = manager("disk_info_relative");
        let err = partition_info("raw:image.raw", &mut manager).unwrap_err();

        assert!(matches!(
            err,
            DiskError::Source(AccessorError::Location { reason, .. }) if reason.contains("absolute")
        ));
    }
}
