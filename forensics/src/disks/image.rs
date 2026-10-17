use crate::{
    accessor::{
        config::AccessorConfig, disk::source::DiskSource, error::AccessorError,
        location::loc::Location,
    },
    disks::{error::DiskResult, partition::disk_format},
    output::{
        manager::OutputManager,
        record::{SingleRecordStream, serialize_to_record},
    },
};

/// Serialize disk image info
pub(crate) fn image_info(source: &str, manager: &mut OutputManager) -> DiskResult<()> {
    let location = Location::parse_source(source)?;
    let format = disk_format(location.scheme)?;

    let image = location.source.ok_or_else(|| AccessorError::Location {
        input: source.to_string(),
        reason: String::from("Disk image paths must be absolute"),
    })?;

    let meta = DiskSource::open(&AccessorConfig::default(), format, image.as_path())?.image()?;

    let record = serialize_to_record(meta)?;
    manager.write_artifact("disk_image", &"", &mut SingleRecordStream::new(record))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use crate::{
        accessor::error::AccessorError,
        disks::{error::DiskError, image::image_info},
        output::manager::OutputManager,
        structs::toml::{OutputConfig, OutputDestination, OutputFormat},
    };
    use std::{fs, path::PathBuf};

    fn test_image() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/disks/qcow/ntfs.qcow");
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
    fn test_image_info() {
        let image = test_image();
        let source = format!("qcow:{}", image.display());
        let mut manager = manager("disk_image");
        image_info(&source, &mut manager).unwrap();

        manager.finalize().unwrap();
        let output_dir = PathBuf::from("./tmp/disk_image");
        let mut jsonl = String::new();
        for entry in fs::read_dir(output_dir).unwrap() {
            let file = entry.unwrap().path();
            let filename = file.file_name().unwrap().to_string_lossy();
            if filename.starts_with("disk_image_") && filename.ends_with(".jsonl") {
                jsonl.push_str(&fs::read_to_string(file).unwrap());
            }
        }

        let record: Value = serde_json::from_str(jsonl.lines().next().unwrap()).unwrap();
        assert_eq!(record["format"], "Qcow");
        assert_eq!(
            record["file_size"].as_u64(),
            Some(fs::metadata(image).unwrap().len())
        );

        assert_eq!(record["virtual_size"], 8_000_000);
        let qcow = &record["details"]["Qcow"];
        assert_eq!(qcow["version"], "V3");
        assert_eq!(qcow["cluster_bits"], 16);
        assert_eq!(qcow["cluster_size"], 65536);
        assert_eq!(qcow["l1_entries"], 1);
        assert_eq!(qcow["l1_offset"], 196608);

        assert_eq!(qcow["refcount_offset"], 65536);
        assert_eq!(qcow["refcount_clusters"], 1);
        assert_eq!(qcow["refcount_order"], 4);
        assert_eq!(qcow["snapshot_count"], 0);
        assert_eq!(qcow["snapshot_offset"], 0);
        assert_eq!(qcow["encryption"], "None");
        assert!(qcow["backing_file"].is_null());
        assert_eq!(qcow["compression"], "Zlib");

        let flags = &qcow["flags"];
        assert_eq!(flags["incompatible"], 0);
        assert_eq!(flags["compatible"], 0);
        assert_eq!(flags["autoclear"], 0);
        assert_eq!(flags["dirty"], false);

        assert_eq!(flags["corrupt"], false);
        assert_eq!(flags["external_data"], false);
        assert_eq!(flags["compression_type"], false);
        assert_eq!(flags["extended_l2"], false);

        assert_eq!(flags["lazy_refcounts"], false);
        assert_eq!(flags["bitmaps"], false);
        assert_eq!(flags["raw_external_data"], false);
        assert_eq!(record["collection_metadata"]["artifact_name"], "disk_image");
    }

    #[test]
    fn test_image_info_relative_source_is_error() {
        let mut manager = manager("disk_image_relative");
        let err = image_info("qcow:image.qcow", &mut manager).unwrap_err();
        assert!(matches!(
            err,
            DiskError::Source(AccessorError::Location { reason, .. })
                if reason.contains("absolute")
        ));
    }
}
