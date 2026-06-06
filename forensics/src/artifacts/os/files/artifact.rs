use super::{error::FileError, filelisting::get_filelist};
use crate::{output::manager::OutputManager, structs::artifacts::os::files::FileOptions};
use log::error;

/// Get a filelisting based on provided options
pub(crate) fn filelisting(
    manager: &mut OutputManager,
    options: &FileOptions,
) -> Result<(), FileError> {
    if let Err(err) = get_filelist(options, manager) {
        error!("[forensics] Failed to get file listing: {err:?}");
        return Err(FileError::Filelisting);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::files::artifact::filelisting,
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
            metadata: Some(false),
            md5: Some(false),
            sha1: Some(false),
            sha256: Some(false),
            path_regex: None,
            filename_regex: None,
            yara: None,
            exclude_directories: None,
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
            metadata: Some(false),
            md5: Some(false),
            sha1: Some(false),
            sha256: Some(false),
            path_regex: None,
            filename_regex: None,
            yara: None,
            exclude_directories: None,
        };
        let status = filelisting(&mut output, &file_config).unwrap();
        assert_eq!(status, ());
    }
}
