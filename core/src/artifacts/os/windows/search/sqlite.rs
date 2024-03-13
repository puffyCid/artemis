use super::{error::SearchError, ese::SearchEntry};
use crate::{
    artifacts::os::windows::artifacts::output_data, structs::toml::Output, utils::time::time_now,
};
use log::{error, warn};
use rusqlite::{Connection, OpenFlags};
use std::collections::HashMap;

struct SqlEntry {
    document_id: i32,
    value: String,
    prop: String,
}

/// Parse the Windows `Search` SQLITE file
pub(crate) fn parse_search_sqlite(
    path: &str,
    output: &mut Output,
    filter: &bool,
) -> Result<(), SearchError> {
    let start_time = time_now();

    // Bypass SQLITE file lock
    let search_file = format!("file:{path}?immutable=1");

    let connection = Connection::open_with_flags(
        search_file,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    );
    let conn = match connection {
        Ok(connect) => connect,
        Err(err) => {
            error!("[search] Failed to read Search SQLITE file {err:?}");
            return Err(SearchError::SqliteParse);
        }
    };

    let query = "SELECT WorkId,quote(Value) as Value,UniqueKey from SystemIndex_1_PropertyStore join SystemIndex_1_PropertyStore_Metadata on SystemIndex_1_PropertyStore.ColumnId = SystemIndex_1_PropertyStore_Metadata.Id order by SystemIndex_1_PropertyStore.WorkId";
    let statement = conn.prepare(query);
    let mut stmt = match statement {
        Ok(result) => result,
        Err(err) => {
            error!("[search] Failed to compose Search SQL query {err:?}");
            return Err(SearchError::BadSQL);
        }
    };

    let search_data = stmt.query_map([], |row| {
        Ok(SqlEntry {
            document_id: row.get("WorkId")?,
            value: row.get("Value")?,
            prop: row.get("UniqueKey")?,
        })
    });

    match search_data {
        Ok(search_iter) => {
            let mut entries = Vec::new();
            let limit = 100000;
            let mut entry = SearchEntry {
                document_id: 1,
                entry: String::new(),
                last_modified: 0,
                properties: HashMap::new(),
            };
            // Go through each row, while the entry.document_id and sql_entry.document_id are the same each row is a property value.
            // Once the doucment_id is different we have arrived at the next entry
            for search in search_iter {
                match search {
                    Ok(sql_entry) => {
                        if entry.document_id == sql_entry.document_id {
                            entry
                                .properties
                                .insert(sql_entry.prop, sql_entry.value.replace('\'', ""));

                            continue;
                        }

                        entries.push(entry.clone());
                        entry.document_id = sql_entry.document_id;
                        // Now have new properties associated with new document_id
                        entry.properties.clear();

                        entry.properties.insert(sql_entry.prop, sql_entry.value);
                        // We set a limit just in case a system has indexed alot of data
                        if entries.len() == limit {
                            let serde_data_result = serde_json::to_value(&entries);
                            let serde_data = match serde_data_result {
                                Ok(results) => results,
                                Err(err) => {
                                    error!(
                                        "[search] Failed to serialize search SQLITE data: {err:?}"
                                    );
                                    return Err(SearchError::Serialize);
                                }
                            };
                            let result =
                                output_data(&serde_data, "search", output, &start_time, filter);
                            match result {
                                Ok(_result) => {}
                                Err(err) => {
                                    error!("[search] Could not output search SQLITE data: {err:?}");
                                }
                            }

                            entries = Vec::new();
                        }
                    }
                    Err(err) => {
                        warn!("[search] Failed to iterate through Search data: {err:?}");
                    }
                }
            }

            if entries.is_empty() {
                return Ok(());
            }

            // Output any leftover data
            let serde_data_result = serde_json::to_value(&entries);
            let serde_data = match serde_data_result {
                Ok(results) => results,
                Err(err) => {
                    error!("[search] Failed to serialize search SQLITE data: {err:?}");
                    return Err(SearchError::Serialize);
                }
            };
            let result = output_data(&serde_data, "search", output, &start_time, filter);
            match result {
                Ok(_result) => {}
                Err(err) => {
                    error!("[search] Could not output search SQLITE data: {err:?}");
                }
            }
        }
        Err(err) => {
            error!("[search]  Failed to get Search SQLITE data: {err:?}");
            return Err(SearchError::SqliteParse);
        }
    }

    Ok(())
}

/// Parse the Windows `Search` SQLITE file and return results
pub(crate) fn parse_search_sqlite_path(path: &str) -> Result<Vec<SearchEntry>, SearchError> {
    // Bypass SQLITE file lock
    let search_file = format!("file:{path}?immutable=1");

    let connection = Connection::open_with_flags(
        search_file,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    );
    let conn = match connection {
        Ok(connect) => connect,
        Err(err) => {
            error!("[search] Failed to read Search SQLITE file {err:?}");
            return Err(SearchError::SqliteParse);
        }
    };

    let query = "SELECT WorkId,quote(Value) as Value,UniqueKey from SystemIndex_1_PropertyStore join SystemIndex_1_PropertyStore_Metadata on SystemIndex_1_PropertyStore.ColumnId = SystemIndex_1_PropertyStore_Metadata.Id order by SystemIndex_1_PropertyStore.WorkId";
    let statement = conn.prepare(query);
    let mut stmt = match statement {
        Ok(result) => result,
        Err(err) => {
            error!("[search] Failed to compose Search SQL query {err:?}");
            return Err(SearchError::BadSQL);
        }
    };

    let search_data = stmt.query_map([], |row| {
        Ok(SqlEntry {
            document_id: row.get("WorkId")?,
            value: row.get("Value")?,
            prop: row.get("UniqueKey")?,
        })
    });
    let mut entries = Vec::new();

    match search_data {
        Ok(search_iter) => {
            let mut entry = SearchEntry {
                document_id: 1,
                entry: String::new(),
                last_modified: 0,
                properties: HashMap::new(),
            };
            // Go through each row, while the entry.document_id and sql_entry.document_id are the same each row is a property.
            // Once the doucment_id is different we have arrived at the next entry
            for search in search_iter {
                match search {
                    Ok(sql_entry) => {
                        if entry.document_id == sql_entry.document_id {
                            entry
                                .properties
                                .insert(sql_entry.prop, sql_entry.value.replace('\'', ""));

                            continue;
                        }

                        entries.push(entry.clone());
                        entry.document_id = sql_entry.document_id;
                        // Now have new properties associated with new document_id
                        entry.properties.clear();

                        entry.properties.insert(sql_entry.prop, sql_entry.value);
                    }
                    Err(err) => {
                        warn!("[search] Failed to iterate through Search data: {err:?}");
                    }
                }
            }
        }
        Err(err) => {
            error!("[search]  Failed to get Search SQLITE data: {err:?}");
            return Err(SearchError::SqliteParse);
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::{parse_search_sqlite, parse_search_sqlite_path};
    use crate::structs::toml::Output;
    use std::path::PathBuf;

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
    fn test_parse_search_sqlite() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/search/win11/Windows.db");
        let mut output = output_options("search_temp", "local", "./tmp", false);

        parse_search_sqlite(&test_location.display().to_string(), &mut output, &false).unwrap();
    }

    #[test]
    fn test_parse_search_sqlite_path() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/search/win11/Windows.db");

        let results = parse_search_sqlite_path(&test_location.display().to_string()).unwrap();
        assert_eq!(results.len(), 1437);
        assert_eq!(results[1295].properties.get("4447-System_ItemPathDisplay").unwrap(), "C:\\Users\\bob\\.cargo\\registry\\cache\\github.com-1ecc6299db9ec823\\bytecount-0.6.3.crate");
        assert_eq!(
            results[1295]
                .properties
                .get("4365-System_DateImported")
                .unwrap(),
            "X0917F6B09D44D901"
        );
    }
}
