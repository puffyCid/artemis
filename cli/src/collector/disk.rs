use clap::{Args, Subcommand};
use forensics::{
    disks::disk::{disk_boot, disk_filesystem, disk_partition},
    structs::toml::{OutputConfig, OutputFormat},
};
use std::path::PathBuf;

/// Output flags used by the disk commands
#[derive(Args, Debug)]
pub(crate) struct DiskOutput {
    /// Directory for results
    #[arg(long, default_value_t = String::from("./tmp"))]
    output_dir: String,
    /// GZIP compress results
    #[arg(long)]
    compress: bool,
}

#[derive(Subcommand, Debug)]
pub(crate) enum DiskCommands {
    /// Logical disk layout
    Partition {
        /// Disk image source. Example: raw:/absolute/path/image.raw
        #[arg(long)]
        source: String,
        /// Flags for output
        #[command(flatten)]
        output: DiskOutput,
    },
    /// Boot sector and partition table
    Bootloader {
        /// Disk image source. Example: raw:/absolute/path/image.raw
        #[arg(long)]
        source: String,
        /// Flags for output
        #[command(flatten)]
        output: DiskOutput,
    },
    /// Filesystem details for each partition
    Filesystem {
        /// Disk image source. Example: raw:/absolute/path/image.raw
        #[arg(long)]
        source: String,
        /// Flags for output
        #[command(flatten)]
        output: DiskOutput,
    },
}

/// Run support disk commands
pub(crate) fn run_disk(command: &DiskCommands, mut config: OutputConfig) {
    let result = match command {
        DiskCommands::Partition { source, output } => {
            disk_output(&mut config, output);
            disk_partition(source, config)
        }
        DiskCommands::Bootloader { source, output } => {
            disk_output(&mut config, output);
            disk_boot(source, config)
        }
        DiskCommands::Filesystem { source, output } => {
            disk_output(&mut config, output);
            disk_filesystem(source, config)
        }
    };

    if let Err(err) = result {
        println!("Failed to query disk: {err}");
    }
}

/// For now disk info commands writes only to JSON
fn disk_output(output: &mut OutputConfig, disk: &DiskOutput) {
    output.compress = disk.compress;
    output.format = OutputFormat::Json;
    if !disk.output_dir.is_empty() {
        output.directory = PathBuf::from(&disk.output_dir);
    }
    println!("[artemis] Writing output to: {:?}", output.directory);
}

#[cfg(test)]
mod tests {
    use super::{DiskCommands, run_disk};
    use forensics::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use std::{fs, path::PathBuf};

    fn test_image() -> PathBuf {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("../forensics/tests/test_data/filesystems/ntfs/test.raw");
        path
    }

    fn output() -> OutputConfig {
        OutputConfig {
            name: String::from("disk_cli"),
            directory: PathBuf::from("./tmp"),
            format: OutputFormat::Json,
            compress: false,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        }
    }

    fn json_contains(stem: &str, expected: &str) {
        let output_dir = PathBuf::from("./tmp/disk_cli");
        let mut json = String::new();

        for entry in fs::read_dir(&output_dir).unwrap() {
            let file = entry.unwrap().path();
            let filename = file.file_name().unwrap().to_string_lossy();
            if filename.starts_with(stem) && filename.ends_with(".json") {
                json.push_str(&fs::read_to_string(&file).unwrap());
            }
        }

        assert!(json.contains(expected), "{stem} missing {expected}");
    }

    #[test]
    fn test_run_disk_commands() {
        let source = format!("raw:{}", test_image().display());
        let _ = fs::remove_dir_all("./tmp/disk_cli");

        let commands = [
            DiskCommands::Partition {
                source: source.clone(),
                output: super::DiskOutput {
                    output_dir: String::from("./tmp"),
                    compress: false,
                },
            },
            DiskCommands::Bootloader {
                source: source.clone(),
                output: super::DiskOutput {
                    output_dir: String::from("./tmp"),
                    compress: false,
                },
            },
            DiskCommands::Filesystem {
                source,
                output: super::DiskOutput {
                    output_dir: String::from("./tmp"),
                    compress: false,
                },
            },
        ];

        for command in &commands {
            run_disk(command, output());
        }

        json_contains("disk_info_", "Partition0");
        json_contains("disk_boot_", "\"boot_type\":\"none\"");
        json_contains("disk_filesystem_ntfs_", "\"volume_name\":\"test\"");
    }

    #[test]
    fn test_run_disk_relative_source_is_error() {
        run_disk(
            &DiskCommands::Partition {
                source: String::from("raw:image.raw"),
                output: super::DiskOutput {
                    output_dir: String::from("./tmp"),
                    compress: false,
                },
            },
            output(),
        );
    }
}
