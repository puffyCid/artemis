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

pub fn disk_partition(source: &str, output: OutputConfig) -> Result<(), DiskCommandError> {
    run(output, |manager| partition_info(source, manager))
}

pub fn disk_boot(source: &str, output: OutputConfig) -> Result<(), DiskCommandError> {
    run(output, |manager| boot_info(source, manager))
}

pub fn disk_filesystem(source: &str, output: OutputConfig) -> Result<(), DiskCommandError> {
    run(output, |manager| filesystem_info(source, manager))
}

fn run(
    output: OutputConfig,
    write: impl FnOnce(&mut OutputManager) -> Result<(), DiskError>,
) -> Result<(), DiskCommandError> {
    let mut manager = OutputManager::new(output)?;
    write(&mut manager)?;

    manager.finalize()?;

    Ok(())
}
