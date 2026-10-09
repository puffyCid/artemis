use crate::{
    disks::{
        bootloader::boot_info,
        error::{DiskCommandError, DiskError},
        filesystem::filesystem_info,
        partition::partition_info,
    },
    output::manager::OutputManager,
    structs::toml::OutputConfig,
};

/// View partitions in the image
pub fn disk_partition(source: &str, output: OutputConfig) -> Result<(), DiskCommandError> {
    run(output, |manager| partition_info(source, manager))
}

/// View bootsector information
pub fn disk_boot(source: &str, output: OutputConfig) -> Result<(), DiskCommandError> {
    run(output, |manager| boot_info(source, manager))
}

/// View filesystem details
pub fn disk_filesystem(source: &str, output: OutputConfig) -> Result<(), DiskCommandError> {
    run(output, |manager| filesystem_info(source, manager))
}

/// Execute the provided disk command
fn run(
    output: OutputConfig,
    write: impl FnOnce(&mut OutputManager) -> Result<(), DiskError>,
) -> Result<(), DiskCommandError> {
    let mut manager = OutputManager::new(output)?;
    write(&mut manager)?;

    manager.finalize()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        disks::disk::{disk_boot, disk_filesystem, disk_partition},
        structs::toml::{OutputConfig, OutputDestination, OutputFormat},
    };
    use std::path::PathBuf;

    fn test_image() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("forensics/tests/test_data/filesystems/ntfs/test.raw");
        path
    }

    fn output() -> OutputConfig {
        OutputConfig {
            name: String::from("test_run"),
            directory: PathBuf::from("./tmp"),
            format: OutputFormat::Json,
            compress: false,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        }
    }

    #[test]
    fn test_disk_boot() {
        let source = format!("raw:{}", test_image().display());

        disk_boot(&source, output()).unwrap();
    }

    #[test]
    fn test_disk_partition() {
        let source = format!("raw:{}", test_image().display());

        disk_partition(&source, output()).unwrap();
    }

    #[test]
    fn test_disk_filesystem() {
        let source = format!("raw:{}", test_image().display());

        disk_filesystem(&source, output()).unwrap();
    }
}
