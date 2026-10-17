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
    use crate::{
        disks::image::image_info,
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
    }
}
