/**
 * Windows `Search` is an indexing service for tracking a huge amount of files and content on Windows.  
 * `Search` can parse a large amount of metadata (properties) for each entry it indexes. It has almost 600 different types of properties it can parse.
 * It can even index some of the contents of a file.  
 *
 * `Search` can index large parts of the file system, so parsing the `Search` database can provide a partial file listing of the system.
 * `Search` is disabled on Windows Servers and starting on newer versions of Windows 11 it is stored in a SQLITE database (previously was an ESE database)
 *
 * References:  
 * `https://github.com/libyal/esedb-kb/blob/main/documentation/Windows%20Search.asciidoc`
 * `https://en.wikipedia.org/wiki/Windows_Search`
 *
 * Other parsers:  
 * `https://github.com/strozfriedberg/sidr`
 * `https://github.com/moaistory/WinSearchDBAnalyzer`
 * `https://github.com/libyal/libesedb`
 */
use super::{
    error::SearchError,
    ese::{SearchEntry, parse_search, parse_search_path},
    sqlite::{parse_search_sqlite, parse_search_sqlite_path},
};
use crate::{
    accessor::{
        access::Accessor,
        entry::handle::{EntryKind, FileHandle},
    },
    output::manager::OutputManager,
    structs::artifacts::os::windows::SearchOptions,
    utils::environment::get_systemdrive,
};
use tracing::error;

/// Grab the Windows `Search` data
pub(crate) fn grab_search(
    options: &SearchOptions,
    manager: &mut OutputManager,
) -> Result<(), SearchError> {
    let pattern = if let Some(file) = &options.alt_file {
        file.clone()
    } else {
        let drive = match get_systemdrive() {
            Ok(result) => result,
            Err(err) => {
                error!("Could not get systemdrive: {err:?}");
                return Err(SearchError::Systemdrive);
            }
        };
        format!(
            "ntfs:{drive}:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows*"
        )
    };

    let mut accessor = Accessor::with_defaults();
    let paths = match accessor.globfs(&pattern) {
        Ok(results) => results,
        Err(err) => {
            error!("Could not glob Windows Search {pattern}: {err:?}");
            return Err(SearchError::Systemdrive);
        }
    };

    for entry in paths {
        if entry.meta.kind != EntryKind::File {
            continue;
        }

        let Some(handle) = entry.handle.as_file() else {
            continue;
        };

        if handle.display_path().ends_with("Windows.edb") {
            parse_search(handle, manager, options);
            continue;
        }
        // If we do not find Windows.edb we may be dealing with Windows 11 db
        /*
         * Windows Search on Windows 11 is split into three (3) SQLITE databases:
         *  - Windows.db
         *  - Windows-usn.db
         *  - Windows-gther.db
         *
         * Windows.db contains the metadata on indexed files
         * Windows-gther.db contains the indexed file entry.
         * Unsure what Windows-usn.db is used for.
         *
         * Windows-gthr.db is created with a special SQLITE collating feature that requires a custom SQLITE callback function to handle: "UNICODE_en-US_LINGUISTIC_IGNORECASE".
         * Basically we need to create a function to handle string comparisons for Windows-gthr.db before we are allowed to query it.
         * We do not do that, instead we just parse the Windows.db file which often contains enough metadata to figure out what the entry is.
         *
         * References:
         * `https://www.sqlite.org/datatype3.html#collation`
         * `https://github.com/strozfriedberg/sidr/blob/main/src/sqlite.rs#L14`
         */
        parse_search_sqlite(handle, manager, options);
    }

    Ok(())
}

/// Parse a provided Windows `Search` file and return its contents
pub(crate) fn grab_search_path(
    handle: &FileHandle,
    page_limit: u32,
) -> Result<Vec<SearchEntry>, SearchError> {
    if handle.display_path().ends_with("Windows.edb") {
        return parse_search_path(handle, page_limit);
    } else if handle.display_path().ends_with("Windows.db") {
        parse_search_sqlite_path(handle)?;
    }
    return Err(SearchError::NotSearchFile);
}

#[cfg(test)]
mod tests {
    use super::grab_search;
    use super::grab_search_path;
    use crate::accessor::access::Accessor;
    use crate::filesystem::files::is_file;
    use crate::output::manager::OutputManager;
    use crate::structs::artifacts::os::windows::SearchOptions;
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
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
    fn test_grab_search() {
        let mut output = output_options("search_temp", "./tmp", false);
        let options = SearchOptions { alt_file: None };

        let _ = grab_search(&options, &mut output);
    }

    #[test]
    #[ignore = "Can take a long time"]
    fn test_grab_search_path() {
        let test_path =
            "C:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb";
        // Some versions of Windows 11 do not use ESE for Windows Search
        if !is_file(test_path) {
            return;
        }
        let binding = Accessor::with_defaults()
            .globfs(&format!("ntfs:{test_path}"))
            .unwrap();
        let handle = binding[0].handle.as_file().unwrap();
        let results = grab_search_path(handle, 50).unwrap();
        assert!(results.len() > 20);
    }
}
