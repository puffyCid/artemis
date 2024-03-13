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
use super::{error::FsEventsError, fsevent::fsevents_data};
use crate::{
    filesystem::files::list_files, structs::artifacts::os::macos::FseventsOptions,
    utils::compression::decompress::decompress_gzip,
};
use common::macos::FsEvents;
use log::error;

/// Parse `FsEvent` files. Check for `/System/Volumes/Data/.fseventsd/` and `/.fseventsd` paths
pub(crate) fn grab_fseventsd(options: &FseventsOptions) -> Result<Vec<FsEvents>, FsEventsError> {
    if let Some(alt_file) = &options.alt_file {
        return grab_fsventsd_file(alt_file);
    }

    let mut events = get_fseventsd()?;
    let legacy = get_fseventsd_legacy();
    if let Ok(mut results) = legacy {
        events.append(&mut results);
    }

    let mut fsevents_data: Vec<FsEvents> = Vec::new();
    for file in events {
        let decompress_result = decompress_gzip(&file);
        let decompress_data = match decompress_result {
            Ok(result) => result,
            Err(err) => {
                error!("[fsevent] Could not decompress data {err:?}");
                return Err(FsEventsError::Decompress);
            }
        };

        let results = parse_fsevents(&decompress_data);
        match results {
            Ok((_, mut data)) => fsevents_data.append(&mut data),
            Err(err) => error!("Failed to parse FsEvent file {file}, err: {err:?}"),
        }
    }
    Ok(fsevents_data)
}

/// Parse a single `FsEvent` file
pub(crate) fn grab_fsventsd_file(path: &str) -> Result<Vec<FsEvents>, FsEventsError> {
    let mut fsevents_data: Vec<FsEvents> = Vec::new();
    let decompress_result = decompress_gzip(path);
    let decompress_data = match decompress_result {
        Ok(result) => result,
        Err(err) => {
            error!("[fsevent] Could not decompress data {err:?}");
            return Err(FsEventsError::Decompress);
        }
    };

    let results = parse_fsevents(&decompress_data);
    match results {
        Ok((_, mut data)) => fsevents_data.append(&mut data),
        Err(err) => error!("Failed to parse FsEvent file {path}, err: {err:?}"),
    }

    Ok(fsevents_data)
}

/// Get `FsEvent` files at default path
fn get_fseventsd() -> Result<Vec<String>, FsEventsError> {
    let path = "/System/Volumes/Data/.fseventsd/";
    fseventsd(path)
}

/// Get `FsEvent` files at old path
fn get_fseventsd_legacy() -> Result<Vec<String>, FsEventsError> {
    let path = "/.fseventsd";
    fseventsd(path)
}

/// Get `FsEvents` data from decompressed file
fn parse_fsevents(data: &[u8]) -> nom::IResult<&[u8], Vec<FsEvents>> {
    fsevents_data(data)
}

/// Get list of `FsEvents` files in a directory
fn fseventsd(directory: &str) -> Result<Vec<String>, FsEventsError> {
    let files_result = list_files(directory);
    let fsevent_files = match files_result {
        Ok(result) => result,
        Err(err) => {
            error!("[fsevents] Could not list FsEvents files: {err:?}");
            return Err(FsEventsError::Files);
        }
    };

    let mut files: Vec<String> = Vec::new();
    // read all files under `FsEvents` directory
    // Skip fseventsd-uuid because it is not a `FsEvents` file
    for file_path in fsevent_files {
        if file_path.contains("fseventsd-uuid") {
            continue;
        }
        files.push(file_path);
    }
    Ok(files)
}

#[cfg(test)]
#[cfg(target_os = "macos")]
mod tests {
    use super::{fseventsd, grab_fseventsd, parse_fsevents};
    use crate::{
        artifacts::os::macos::fsevents::parser::{get_fseventsd, grab_fsventsd_file},
        structs::artifacts::os::macos::FseventsOptions,
        utils::compression::decompress::decompress_gzip,
    };
    use std::path::PathBuf;

    #[test]
    fn test_get_fseventsd() {
        let files = get_fseventsd().unwrap();
        assert!(files.len() > 3);
    }

    #[test]
    fn test_grab_fseventsd() {
        let results = grab_fseventsd(&FseventsOptions { alt_file: None }).unwrap();
        assert!(results.len() > 100);
    }

    #[test]
    fn test_fseventsd() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/fsevents/DLS2/");
        let files = fseventsd(&test_location.display().to_string()).unwrap();
        assert_eq!(files.len(), 2)
    }

    #[test]
    fn test_parse_fsevents() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/fsevents/DLS2/0000000000027d79");
        let test_path: &str = &test_location.display().to_string();
        let files = decompress_gzip(test_path).unwrap();
        let (_, results) = parse_fsevents(&files).unwrap();
        assert_eq!(results.len(), 736)
    }

    #[test]
    fn test_grab_fsventsd_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/fsevents/DLS2/0000000000027d79");
        let test_path = &test_location.display().to_string();
        let results = grab_fsventsd_file(test_path).unwrap();
        assert_eq!(results.len(), 736)
    }

    #[test]
    #[should_panic(expected = "Eof")]
    fn test_malformed() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/fsevents/Malformed/malformed");
        let test_path: &str = &test_location.display().to_string();
        let files = decompress_gzip(test_path).unwrap();
        let _results = parse_fsevents(&files).unwrap();
    }

    #[test]
    fn test_parse_fsevents_version1() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/fsevents/DLS1/0000000000027d7a");
        let test_path: &str = &test_location.display().to_string();
        let files = decompress_gzip(test_path).unwrap();
        let (_, results) = parse_fsevents(&files).unwrap();

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].path, "/.fseventsd/sl-compat");
        assert_eq!(results[0].event_id, 163194);
        assert_eq!(results[0].flags, ["IsDirectory"]);
        assert_eq!(results[0].node, 0);
    }
}
