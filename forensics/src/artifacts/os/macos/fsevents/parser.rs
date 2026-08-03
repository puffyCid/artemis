/**
 * macOS `FsEvent` data track changes to files on a system (similar to `UsnJrnl`)  
 * Resides at `/System/Volumes/Data/.fseventsd/` or `/.fseventsd` on older systems
 *
 * References:  
 *   `https://github.com/libyal/dtformats/blob/main/documentation/MacOS%20File%20System%20Events%20Disk%20Log%20Stream%20format.asciidoc`  
 *   `http://www.osdfcon.org/presentations/2017/Ibrahim-Understanding-MacOS-File-Ststem-Events-with-FSEvents-Parser.pdf`
 *
 * Other Parsers:  
 *   `https://github.com/Velocidex/velociraptor`
 */
use super::error::FsEventsError;
use crate::{
    accessor::{access::Accessor, entry::handle::EntryKind},
    artifacts::os::macos::fsevents::fsevent::extract_fsevents,
    output::{manager::OutputManager, record::serialize_records_to_stream},
    structs::artifacts::os::macos::FseventsOptions,
};
use common::macos::FsEvents;
use tracing::{error, warn};

/// Parse `FsEvent` files. Check for `/System/Volumes/Data/.fseventsd/` and `/.fseventsd` paths
pub(crate) fn grab_fseventsd(
    options: &FseventsOptions,
    manager: &mut OutputManager,
) -> Result<(), FsEventsError> {
    let paths = if let Some(alt_path) = &options.alt_file {
        vec![alt_path.as_str()]
    } else {
        vec!["/System/Volumes/Data/.fseventsd/*", "/.fseventsd/*"]
    };

    let mut accessor = Accessor::with_defaults();
    for path in paths {
        let files = match accessor.globfs(path) {
            Ok(result) => result,
            Err(err) => {
                warn!("Could not glob '{path}: {err:?}");
                continue;
            }
        };

        for file in files {
            if file.meta.kind != EntryKind::File
                || file.meta.display_path.ends_with("fseventsd-uuid")
            {
                continue;
            }

            let Some(file_handle) = file.handle.as_file() else {
                continue;
            };

            let bytes = match accessor.read_file_handle(file_handle) {
                Ok(result) => result,
                Err(err) => {
                    warn!("Could not read '{path}': {err:?}");
                    continue;
                }
            };

            let events = match extract_fsevents(bytes, file_handle.display_path()) {
                Ok(result) => result,
                Err(err) => {
                    warn!("Could not parse '{path}': {err:?}");
                    continue;
                }
            };

            if let Err(err) = output_fsevents(events, manager, options) {
                warn!("Could not write fsevents output for '{path}': {err:?}");
            }
        }
    }

    Ok(())
}

/// Parse a single `FsEvent` file
pub(crate) fn grab_fsventsd_file(path: &str) -> Result<Vec<FsEvents>, FsEventsError> {
    let bytes = match Accessor::with_defaults().read_file(path) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not read file '{path}': {err:?}");
            return Err(FsEventsError::Files);
        }
    };
    let events = match extract_fsevents(bytes, path.to_string()) {
        Ok(result) => result,
        Err(err) => {
            error!("Could not parse '{path}': {err:?}");
            return Err(FsEventsError::Files);
        }
    };

    Ok(events)
}

/// Output `FsEvents` results
fn output_fsevents(
    entries: Vec<FsEvents>,
    manager: &mut OutputManager,
    options: &FseventsOptions,
) -> Result<(), FsEventsError> {
    if entries.is_empty() {
        return Ok(());
    }
    let mut records = match serialize_records_to_stream(entries) {
        Ok(results) => results,
        Err(err) => {
            error!("[fsevent] Failed to serialize fsevents entries: {err:?}");
            return Err(FsEventsError::Serialize);
        }
    };

    let artifact_name = "fseventsd";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[fsevent] Could not output fsevents data: {err:?}");
        return Err(FsEventsError::OutputData);
    }

    Ok(())
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use crate::{
        artifacts::os::macos::fsevents::parser::{grab_fseventsd, grab_fsventsd_file},
        output::manager::OutputManager,
        structs::{
            artifacts::os::macos::FseventsOptions,
            toml::{OutputConfig, OutputDestination, OutputFormat},
        },
    };
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputConfig {
        OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Csv,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        }
    }

    #[test]
    fn test_grab_fseventsd() {
        let output = output_options("fsevents_test", "./tmp", false);
        let mut manage = OutputManager::new(output).unwrap();
        grab_fseventsd(&FseventsOptions { alt_file: None }, &mut manage).unwrap();
    }

    #[test]
    fn test_fseventsd() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/fsevents/DLS2/");
        let output = output_options("fsevents_test", "./tmp", false);
        let mut manage = OutputManager::new(output).unwrap();
        grab_fseventsd(
            &FseventsOptions {
                alt_file: Some(test_location.display().to_string()),
            },
            &mut manage,
        )
        .unwrap();
    }

    #[test]
    fn test_grab_fsventsd_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/fsevents/DLS2/0000000000027d79");
        let test_path = &test_location.display().to_string();
        let results = grab_fsventsd_file(test_path).unwrap();
        assert_eq!(results.len(), 736)
    }
}
