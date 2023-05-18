use super::{
    error::SearchError,
    properties::parse_prop_id_lookup,
    tables::indexgthr::{parse_index_gthr, parse_index_gthr_path},
};
use crate::{
    artifacts::os::windows::ese::parser::grab_ese_tables_path,
    utils::{artemis_toml::Output, time::time_now},
};
use log::{error, warn};
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize, Debug, Clone)]
pub(crate) struct SearchEntry {
    pub(crate) document_id: i32,
    pub(crate) entry: String,
    pub(crate) last_modified: i64,
    pub(crate) properties: HashMap<String, String>,
}

/// Parse the Windows `Search` ESE database
pub(crate) fn parse_search(
    path: &str,
    tables: &[String],
    output: &mut Output,
    filter: &bool,
) -> Result<(), SearchError> {
    let start_time = time_now();

    let table_results = grab_ese_tables_path(path, tables);
    let table_data = match table_results {
        Ok(results) => results,
        Err(err) => {
            error!("[search] Failed to parse {path} ESE file: {err:?}");
            return Err(SearchError::ParseEse);
        }
    };

    let indexes = if let Some(values) = table_data.get("SystemIndex_PropertyStore") {
        values
    } else {
        warn!("[search] Could not get table SystemIndex_PropertyStore from ESE results");
        return Err(SearchError::MissingIndexes);
    };

    // Grab hashmap that tracks the unique Search entry and their properties
    let props = parse_prop_id_lookup(indexes);
    for table in tables {
        let search_table = if let Some(values) = table_data.get(table) {
            values
        } else {
            warn!("[search] Could not get table {table} from ESE results");
            continue;
        };

        // There are lots of tables in Windows Search, but the most interesting one is SystemIndex_Gthr
        match table.as_str() {
            "SystemIndex_Gthr" => {
                let _ = parse_index_gthr(search_table, &props, output, &start_time, filter);
            }
            _ => continue,
        }
    }

    Ok(())
}

pub(crate) fn parse_search_path(
    path: &str,
    tables: &[String],
) -> Result<Vec<SearchEntry>, SearchError> {
    let table_results = grab_ese_tables_path(path, tables);
    let table_data = match table_results {
        Ok(results) => results,
        Err(err) => {
            error!("[search] Failed to parse {path} ESE file: {err:?}");
            return Err(SearchError::ParseEse);
        }
    };

    let indexes = if let Some(values) = table_data.get("SystemIndex_PropertyStore") {
        values
    } else {
        warn!("[search] Could not get table SystemIndex_PropertyStore from ESE results");
        return Err(SearchError::MissingIndexes);
    };

    // Grab hashmap that tracks the unique Search entry and their properties
    let props = parse_prop_id_lookup(indexes);
    let mut entries: Vec<SearchEntry> = Vec::new();
    for table in tables {
        let search_table = if let Some(values) = table_data.get(table) {
            values
        } else {
            warn!("[search] Could not get table {table} from ESE results");
            continue;
        };

        // There are lots of tables in Windows Search, but the most interesting one is SystemIndex_Gthr
        match table.as_str() {
            "SystemIndex_Gthr" => {
                let _ = parse_index_gthr_path(search_table, &props, &mut entries);
            }
            _ => continue,
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::{parse_search, parse_search_path};
    use crate::{filesystem::files::is_file, utils::artemis_toml::Output};

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            // url: Some(String::new()),
            // port: Some(0),
            // api_key: Some(String::new()),
            // username: Some(String::new()),
            // password: Some(String::new()),
            // generic_keys: Some(Vec::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
        }
    }

    #[test]
    #[ignore = "Can take a long time"]
    fn test_parse_search() {
        let test_path =
            "C:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb";
        // Some versions of Windows 11 do not use ESE for Windows Search
        if !is_file(test_path) {
            return;
        }
        let mut output = output_options("search_temp", "local", "./tmp", false);

        let table = vec![
            String::from("SystemIndex_Gthr"),
            String::from("SystemIndex_PropertyStore"),
        ];

        parse_search(test_path, &table, &mut output, &false).unwrap();
    }

    #[test]
    #[ignore = "Can take a long time"]
    fn test_parse_search_path() {
        let test_path =
            "C:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb";
        // Some versions of Windows 11 do not use ESE for Windows Search
        if !is_file(test_path) {
            return;
        }

        let table = vec![
            String::from("SystemIndex_Gthr"),
            String::from("SystemIndex_PropertyStore"),
        ];

        let results = parse_search_path(test_path, &table).unwrap();
        assert!(results.len() > 20);
    }
}
