/**
 * Get a standard filelisting from the system.
 * Supports both Windows and macOS, in addition can parse executable metadata for each OS based on `FileOptions`
 * `PE` for Windows
 * `MACHO` for macOS
 * `ELF` for Linux
 *
 * On macOS the filelisting will read the firmlinks file at `/usr/share/firmlinks` and skip firmlink paths
 */
use super::error::FileError;
use crate::accessor::access::Accessor;
use crate::accessor::entry::locator::SourceId;
use crate::output::manager::OutputManager;
use crate::structs::artifacts::os::files::FileOptions;
use tracing::error;

#[cfg(feature = "yarax")]
use crate::utils::yara::extract_rule;

/// Grab filelisting based on `FileOptions` provided
pub(crate) fn get_filelist(
    options: &FileOptions,
    manager: &mut OutputManager,
) -> Result<(), FileError> {
    let mut accessor = Accessor::with_defaults();

    let source = match accessor.open_source(&options.source) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not open source: {err:?}");
            return Err(FileError::Filelisting);
        }
    };

    let mut rule = String::new();
    #[cfg(feature = "yarax")]
    if options.yara.as_ref().is_some_and(|s| !s.is_empty()) {
        // Unwrap is safe since we validate above
        rule = match extract_rule(options.yara.as_ref().unwrap()) {
            Ok(result) => result,
            Err(err) => {
                error!("Bad yara rule {err:?}");
                return Err(FileError::Filelisting);
            }
        };
    }

    match source.id() {
        SourceId::Ntfs(_) => accessor
            .source_walk_ntfs(&source, options, manager, &rule)
            .map_err(|err| {
                error!("NTFS filelisting failed: {err:?}");
                FileError::Filelisting
            }),
        SourceId::Host => accessor
            .source_walk_host(&source, options, manager, &rule)
            .map_err(|err| {
                error!("Host filelisting failed: {err:?}");
                FileError::Filelisting
            }),
        SourceId::Zip(_) => accessor
            .source_walk_zip(&source, options, manager, &rule)
            .map_err(|err| {
                error!("ZIP filelisting failed: {err:?}");
                FileError::Filelisting
            }),
        SourceId::Disk { .. } => accessor
            .source_walk_disk(&source, options, manager, &rule)
            .map_err(|err| {
                error!("Disk filelisting failed: {err:?}");
                FileError::Filelisting
            }),
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::files::filelisting::get_filelist;
    use crate::{
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
    fn test_get_filelist_unix() {
        let options = FileOptions {
            start_path: String::from("/"),
            source: String::from("host:"),
            ..Default::default()
        };

        let mut output = output_options("unix_root", "./tmp", false);

        get_filelist(&options, &mut output).unwrap();
    }

    #[test]
    fn test_get_filelist_zip() {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.push("tests/test_data/archives/artifacts.zip");

        let options = FileOptions {
            start_path: String::from("/"),
            source: format!("zip:{}", dir.to_str().unwrap()),
            ..Default::default()
        };

        let mut output = output_options("zip_root", "./tmp", false);

        get_filelist(&options, &mut output).unwrap();
    }

    #[test]
    fn test_get_filelist_disk_raw() {
        let mut dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        dir.push("tests/test_data/filesystems/ntfs/test.raw");

        let options = FileOptions {
            start_path: String::from("/"),
            source: format!("raw:{}", dir.to_str().unwrap()),
            ..Default::default()
        };

        let mut output = output_options("raw_oot", "./tmp", false);

        get_filelist(&options, &mut output).unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_filelist_windows() {
        let options = FileOptions {
            start_path: String::from("C:\\"),
            source: String::from("ntfs:C"),
            ..Default::default()
        };

        let mut output = output_options("windows_root", "./tmp", false);

        get_filelist(&options, &mut output).unwrap();
    }
}
