use super::{error::SearchError, ese::SearchEntry};
use crate::{
    accessor::{access::Accessor, entry::handle::FileHandle},
    output::{manager::OutputManager, record::serialize_records_to_stream},
    structs::artifacts::os::windows::SearchOptions,
};
use rusqlite::{Connection, MAIN_DB, OpenFlags};
use std::{collections::HashMap, io::Cursor, mem::take};
use tracing::{error, warn};

struct SqlEntry {
    document_id: i32,
    value: String,
    prop: String,
}

/// Parse the Windows `Search` SQLITE file
pub(crate) fn parse_search_sqlite(
    handle: &FileHandle,
    manager: &mut OutputManager,
    options: &SearchOptions,
) -> Result<(), SearchError> {
    let mut accessor = Accessor::with_defaults();
    let bytes = match accessor.read_file_handle(handle) {
        Ok(results) => results,
        Err(err) => {
            error!(
                "Failed to read Search SQLITE file {} {err:?}",
                handle.display_path()
            );
            return Err(SearchError::SqliteParse);
        }
    };
    let mut conn = match Connection::open_in_memory_with_flags(OpenFlags::SQLITE_OPEN_READ_ONLY) {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to create in memory SQLITE file {err:?}");
            return Err(SearchError::SqliteParse);
        }
    };
    if let Err(err) = conn.deserialize_read_exact(MAIN_DB, Cursor::new(&bytes), bytes.len(), true) {
        error!("Failed to deserialize Windows Search SQLITE file {err:?}");
        return Err(SearchError::SqliteParse);
    }

    println!("{}", conn.is_readonly(MAIN_DB).unwrap());
    println!("{:?}", &bytes.len());

    let query = "SELECT WorkId,quote(Value) as Value,UniqueKey from SystemIndex_1_PropertyStore join SystemIndex_1_PropertyStore_Metadata on SystemIndex_1_PropertyStore.ColumnId = SystemIndex_1_PropertyStore_Metadata.Id order by SystemIndex_1_PropertyStore.WorkId";
    let statement = conn.prepare(query);
    let mut stmt = match statement {
        Ok(result) => result,
        Err(err) => {
            println!("Failed to compose Search SQL query {err:?}");
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
                last_modified: String::from("1970-01-01T00:00:00.000Z"),
                properties: HashMap::new(),
                evidence: handle.display_path(),
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
                            let mut records = match serialize_records_to_stream(take(&mut entries))
                            {
                                Ok(results) => results,
                                Err(err) => {
                                    error!("Failed to serialize search SQLITE data: {err:?}");
                                    continue;
                                }
                            };
                            let artifact_name = "search";
                            if let Err(err) =
                                manager.write_artifact(artifact_name, options, &mut records)
                            {
                                error!("Could not output search SQLITE data: {err:?}");
                            }
                        }
                    }
                    Err(err) => {
                        warn!("Failed to iterate through Search data: {err:?}");
                    }
                }
            }

            if entries.is_empty() {
                return Ok(());
            }

            // Output any leftover data
            let mut records = match serialize_records_to_stream(entries) {
                Ok(results) => results,
                Err(err) => {
                    error!("Failed to serialize remaining search SQLITE data: {err:?}");
                    return Err(SearchError::Serialize);
                }
            };
            let artifact_name = "search";
            if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
                error!("Could not output remaining search SQLITE data: {err:?}");
                return Err(SearchError::Output);
            }
        }
        Err(err) => {
            error!(" Failed to get Search SQLITE data: {err:?}");
            return Err(SearchError::SqliteParse);
        }
    }

    Ok(())
}

/// Parse the Windows `Search` SQLITE file and return results
pub(crate) fn parse_search_sqlite_path(
    handle: &FileHandle,
) -> Result<Vec<SearchEntry>, SearchError> {
    let mut accessor = Accessor::with_defaults();
    let bytes = match accessor.read_file_handle(handle) {
        Ok(results) => results,
        Err(err) => {
            error!(
                "Failed to read Search SQLITE file {} {err:?}",
                handle.display_path()
            );
            return Err(SearchError::SqliteParse);
        }
    };
    let mut conn = match Connection::open_in_memory() {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to create in memory SQLITE file {err:?}");
            return Err(SearchError::SqliteParse);
        }
    };
    if let Err(err) = conn.deserialize_read_exact(MAIN_DB, &bytes[..], bytes.len(), true) {
        error!("Failed to deserialize Windows Search SQLITE file {err:?}");
        return Err(SearchError::SqliteParse);
    }

    let query = "SELECT WorkId,quote(Value) as Value,UniqueKey from SystemIndex_1_PropertyStore join SystemIndex_1_PropertyStore_Metadata on SystemIndex_1_PropertyStore.ColumnId = SystemIndex_1_PropertyStore_Metadata.Id order by SystemIndex_1_PropertyStore.WorkId";
    let statement = conn.prepare(query);
    let mut stmt = match statement {
        Ok(result) => result,
        Err(err) => {
            error!("Failed to compose Search SQL query {err:?}");
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
                last_modified: String::new(),
                properties: HashMap::new(),
                evidence: handle.display_path(),
            };
            // Go through each row, while the entry.document_id and sql_entry.document_id are the same each row is a property.
            // Once the document_id is different we have arrived at the next entry
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
                        warn!("Failed to iterate through Search data: {err:?}");
                    }
                }
            }
        }
        Err(err) => {
            error!(" Failed to get Search SQLITE data: {err:?}");
            return Err(SearchError::SqliteParse);
        }
    }

    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::{parse_search_sqlite, parse_search_sqlite_path};
    use crate::accessor::entry::handle::FileHandle;
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{output::manager::OutputManager, structs::artifacts::os::windows::SearchOptions};
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
    fn test_parse_search_sqlite() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/search/win11/Windows.db");
        let options = SearchOptions { alt_file: None };

        let mut output = output_options("search_temp", "./tmp", false);

        parse_search_sqlite(&FileHandle::host(test_location), &mut output, &options).unwrap();
    }

    #[test]
    fn test_parse_search_sqlite_path() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/search/win11/Windows.db");

        let results = parse_search_sqlite_path(&FileHandle::host(test_location)).unwrap();
        assert_eq!(results.len(), 1437);
        assert_eq!(
            results[1295]
                .properties
                .get("4447-System_ItemPathDisplay")
                .unwrap(),
            "C:\\Users\\bob\\.cargo\\registry\\cache\\github.com-1ecc6299db9ec823\\bytecount-0.6.3.crate"
        );
        assert_eq!(
            results[1295]
                .properties
                .get("4365-System_DateImported")
                .unwrap(),
            "X0917F6B09D44D901"
        );
    }
}
