use crate::{
    accessor::{
        config::AccessorConfig,
        disk::{identify::FilesystemKind, source::DiskSource},
        error::AccessorError,
        filesystem::ntfs::volume::NtfsDetails,
        location::loc::Location,
    },
    disks::{error::DiskResult, partition::disk_format},
    output::{manager::OutputManager, record::serialize_records_to_stream},
};
use serde::Serialize;

/// NTFS volume details for one partition
#[derive(Serialize)]
struct NtfsFilesystem {
    /// Evidence path
    source: String,
    /// Partition ID name
    partition_id: String,
    /// Volume label. Null when the volume has no label
    volume_name: Option<String>,
    /// Major NTFS version
    major_version: u8,
    /// Minor NTFS version
    minor_version: u8,
    /// Raw NTFS volume flags
    volume_flags: u16,
    /// Bytes per sector
    sector_size: u16,
    /// Bytes per cluster
    cluster_size: u32,
    /// Bytes per MFT record
    file_record_size: u32,
    /// NTFS volume length in bytes
    size: u64,
    /// Byte offset of the MFT from the start of the partition
    mft_byte_offset: u64,
    /// NTFS volume serial number, lowercase hex
    serial_number: String,
}

/// Get NTFS volume details for each NTFS partition in the disk image
pub(crate) fn filesystem_info(source: &str, manager: &mut OutputManager) -> DiskResult<()> {
    let location = Location::parse_source(source)?;
    let format = disk_format(location.scheme)?;

    let image = location.source.ok_or_else(|| AccessorError::Location {
        input: source.to_string(),
        reason: String::from("Disk image paths must be absolute"),
    })?;

    let disk = DiskSource::open(&AccessorConfig::default(), format, image.as_path())?;
    let inspect = disk.inspect()?;
    let mut rows = Vec::new();

    for partition in inspect.partitions {
        if partition.filesystem != FilesystemKind::Ntfs {
            continue;
        }
        let details = disk.ntfs_details(&partition.partition)?;
        rows.push(ntfs_filesystem(source, &partition.partition.id, details));
    }

    let mut records = serialize_records_to_stream(rows)?;
    manager.write_output("disk_filesystem_ntfs", &mut records)?;

    Ok(())
}

/// Single NTFS output row
fn ntfs_filesystem(source: &str, partition_id: &str, details: NtfsDetails) -> NtfsFilesystem {
    NtfsFilesystem {
        source: source.to_string(),
        partition_id: partition_id.to_string(),
        volume_name: details.volume_name,
        major_version: details.major_version,
        minor_version: details.minor_version,
        volume_flags: details.volume_flags,
        sector_size: details.sector_size,
        cluster_size: details.cluster_size,
        file_record_size: details.file_record_size,
        size: details.size,
        mft_byte_offset: details.mft_byte_offset,
        serial_number: format!("{:016x}", details.serial_number),
    }
}

#[cfg(test)]
mod tests {
    use super::filesystem_info;
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

    fn output_record(name: &str) -> serde_json::Value {
        let output_dir = PathBuf::from("./tmp").join(name);
        let mut jsonl = String::new();

        for entry in fs::read_dir(&output_dir).unwrap() {
            let file = entry.unwrap().path();
            let filename = file.file_name().unwrap().to_string_lossy();
            if filename.starts_with("disk_filesystem_ntfs_") && filename.ends_with(".jsonl") {
                jsonl.push_str(&fs::read_to_string(&file).unwrap());
            }
        }

        assert_eq!(jsonl.lines().count(), 1);
        serde_json::from_str(jsonl.lines().next().unwrap()).unwrap()
    }

    #[test]
    fn test_filesystem_info_ntfs() {
        let image = test_image();
        let source = format!("raw:{}", image.display());
        let mut manager = manager("disk_filesystem_ntfs");

        filesystem_info(&source, &mut manager).unwrap();
        manager.finalize().unwrap();

        let record = output_record("disk_filesystem_ntfs");

        assert_eq!(record["source"], source);
        assert_eq!(record["partition_id"], "Partition0");
        assert_eq!(record["volume_name"], "test");

        assert_eq!(record["major_version"], 3);
        assert_eq!(record["minor_version"], 1);
        assert_eq!(record["volume_flags"], 0);

        assert_eq!(record["sector_size"], 512);
        assert_eq!(record["cluster_size"], 4096);
        assert_eq!(record["file_record_size"], 1024);
        assert_eq!(record["size"].as_u64(), Some(7999488));

        assert_eq!(record["mft_byte_offset"].as_u64(), Some(16_384));
        assert_eq!(record["serial_number"], "5ac04005779ca2bf");
        assert_eq!(
            record["collection_metadata"]["artifact_name"],
            "disk_filesystem_ntfs"
        );
    }

    #[test]
    fn test_filesystem_info_relative_source_is_error() {
        let mut manager = manager("disk_filesystem_relative");
        let err = filesystem_info("raw:image.raw", &mut manager).unwrap_err();
        assert!(matches!(
            err,
            DiskError::Source(AccessorError::Location { reason, .. }) if reason.contains("absolute")
        ));
    }
}
