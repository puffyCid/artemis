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
    ese::{parse_search, parse_search_path, SearchEntry},
    sqlite::{parse_search_sqlite, parse_search_sqlite_path},
};
use crate::{
    filesystem::files::is_file,
    structs::{artifacts::os::windows::SearchOptions, toml::Output},
    utils::environment::get_systemdrive,
};
use log::error;

/// Grab the Windows `Search` data
pub(crate) fn grab_search(
    options: &SearchOptions,
    output: &mut Output,
    filter: &bool,
) -> Result<(), SearchError> {
    let path = if let Some(alt) = &options.alt_file {
        alt.to_string()
    } else {
        let systemdrive_result = get_systemdrive();
        let systemdrive = match systemdrive_result {
            Ok(result) => result,
            Err(err) => {
                error!("[search] Could not get systemdrive: {err:?}");
                return Err(SearchError::Systemdrive);
            }
        };
        format!("{systemdrive}:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb")
    };

    let win11 = path.replace("edb", "db");

    // If we do not find Windows.edb we may be dealing with Windows 11 db
    if !is_file(&path) && is_file(&win11) {
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
         * Basically we need to create a function to handle string comparisions for Windows-gthr.db before we are allowed to query it.
         * We do not do that, instead we just parse the Windows.db file which often contains enough metadata to figure out what the entry is.
         *
         * References:
         * `https://www.sqlite.org/datatype3.html#collation`
         * `https://github.com/strozfriedberg/sidr/blob/main/src/sqlite.rs#L14`
         */
        return parse_search_sqlite(&win11, output, filter);
    }
    let tables = vec![
        String::from("SystemIndex_Gthr"),
        String::from("SystemIndex_PropertyStore"),
    ];

    parse_search(&path, &tables, output, filter)
}

/// Parse a provided Windows `Search` file and return its contents
pub(crate) fn grab_search_path(path: &str) -> Result<Vec<SearchEntry>, SearchError> {
    let result = if path.ends_with(".edb") {
        let tables = vec![
            String::from("SystemIndex_Gthr"),
            String::from("SystemIndex_PropertyStore"),
        ];
        parse_search_path(path, &tables)?
    } else if path.ends_with(".db") {
        parse_search_sqlite_path(path)?
    } else {
        return Err(SearchError::NotSearchFile);
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::grab_search;
    use super::grab_search_path;
    use crate::filesystem::files::is_file;
    use crate::{structs::artifacts::os::windows::SearchOptions, structs::toml::Output};

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_grab_search() {
        let mut output = output_options("search_temp", "local", "./tmp", false);
        let options = SearchOptions { alt_file: None };

        let _ = grab_search(&options, &mut output, &false);
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

        let results = grab_search_path(test_path).unwrap();
        assert!(results.len() > 20);
    }
}
