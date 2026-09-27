use super::{error::FileError, filelisting::get_filelist};
use crate::accessor::location::loc::Location;
use crate::accessor::location::scheme::Scheme;
use crate::{output::manager::OutputManager, structs::artifacts::os::files::FileOptions};
use tracing::error;

/// Get a filelisting based on provided options
pub(crate) fn filelisting(
    manager: &mut OutputManager,
    options: &FileOptions,
) -> Result<(), FileError> {
    if let Err(err) = get_filelist(options, manager) {
        error!("Failed to get file listing: {err:?}");
        return Err(FileError::Filelisting);
    }

    Ok(())
}

/// If a specific filelisting fails, return a more accurate artifact name besides "files"
pub(crate) fn files_output_name(source: &str) -> &'static str {
    match Location::parse_source(source).map(|loc| loc.scheme) {
        Ok(Scheme::Ntfs) => "files_ntfs",
        Ok(Scheme::Zip) => "files_zip",
        Ok(Scheme::Host) => "files_host",
        Err(_err) => "files",
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::files::artifact::{filelisting, files_output_name},
        output::manager::OutputManager,
        structs::{
            artifacts::os::files::FileOptions,
            toml::{OutputConfig, OutputDestination, OutputFormat},
        },
    };
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_filelisting_unix() {
        let mut output = output_options("file_test", "./tmp", false);

        let file_config = FileOptions {
            start_path: String::from("/"),
            depth: Some(1),
            source: String::from("host:"),
            ..Default::default()
        };
        let status = filelisting(&mut output, &file_config).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_filelisting_windows() {
        let mut output = output_options("file_test", "./tmp", false);

        let file_config = FileOptions {
            start_path: String::from("C:\\"),
            depth: Some(1),
            source: String::from("host:"),
            ..Default::default()
        };
        let status = filelisting(&mut output, &file_config).unwrap();
        assert_eq!(status, ());
    }

    #[test]
    fn test_files_output_name() {
        assert_eq!(files_output_name("ntfs:C:"), "files_ntfs");
        assert_eq!(files_output_name("ntfs:C"), "files_ntfs");
        assert_eq!(files_output_name("zip:/tmp/archive.zip"), "files_zip");

        assert_eq!(files_output_name("host:"), "files_host");
        assert_eq!(files_output_name(""), "files");
        assert_eq!(files_output_name("not-a-source"), "files");
    }

    #[test]
    fn test_failed_ntfs_filelisting_report_name() {
        let mut manager = output_options("files_ntfs_failed", "./tmp", false);
        let options = FileOptions {
            source: String::from("ntfs:C:"),
            ..Default::default()
        };

        manager.write_failed_artifact(files_output_name(&options.source), &options);

        assert_eq!(manager.artifact_runs.len(), 1);
        assert_eq!(manager.artifact_runs[0].name, "files_ntfs");
        assert_eq!(manager.artifact_runs[0].status, "failed");
    }
}
