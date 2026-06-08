use crate::{
    artifacts::os::windows::search::{error::SearchError, ese::SearchEntry},
    output::{manager::OutputManager, record::serialize_records_to_stream},
    structs::artifacts::os::windows::SearchOptions,
    utils::{
        encoding::base64_decode_standard,
        nom_helper::{Endian, nom_unsigned_eight_bytes},
        time::filetime_to_iso,
    },
};
use common::windows::TableDump;
use log::error;
use std::{collections::HashMap, mem::take};

/// Parse the `SystemIndex_Gthr` table and output data
pub(crate) fn parse_index_gthr(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, HashMap<String, String>>,
    manager: &mut OutputManager,
    options: &SearchOptions,
    evidence: &str,
) -> Result<(), SearchError> {
    let mut entries = Vec::new();
    let limit = 100000;

    for rows in column_rows {
        let mut entry = SearchEntry {
            document_id: 0,
            entry: String::new(),
            last_modified: String::from("1970-01-01T00:00:00.000Z"),
            properties: HashMap::new(),
            evidence: evidence.to_string(),
        };

        for column in rows {
            match column.column_name.as_str() {
                "DocumentID" => {
                    entry.document_id = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "LastModified" => {
                    let decode_results = base64_decode_standard(&column.column_data);
                    if let Ok(time_data) = decode_results {
                        if time_data.is_empty() {
                            continue;
                        }

                        // Sometimes the last modified data is just ********
                        let asterisk = 0x2a;
                        if time_data[0] == asterisk {
                            continue;
                        }

                        let time_results = nom_unsigned_eight_bytes(&time_data, Endian::Be);
                        if let Ok((_, result)) = time_results {
                            entry.last_modified = filetime_to_iso(result);
                        }
                    }
                }
                "FileName" => entry.entry.clone_from(&column.column_data),
                _ => (),
            }
        }

        if let Some(props) = lookups.get(&entry.document_id.to_string()) {
            entry.properties.clone_from(props);
        }

        entries.push(entry);

        // We set a limit just in case a system has indexed a lot of data
        if entries.len() == limit {
            let mut records = match serialize_records_to_stream(take(&mut entries)) {
                Ok(results) => results,
                Err(err) => {
                    error!("[search] Failed to serialize search ESE data: {err:?}");
                    continue;
                }
            };
            let artifact_name = "search";
            if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
                error!("[search] Could not output search ESE data: {err:?}");
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
            error!("[search] Failed to serialize remaining search ESE data: {err:?}");
            return Err(SearchError::Serialize);
        }
    };
    let artifact_name = "search";
    if let Err(err) = manager.write_artifact(artifact_name, options, &mut records) {
        error!("[search] Could not output remaining search ESE data: {err:?}");
        return Err(SearchError::Output);
    }

    Ok(())
}

/// Parse the `SystemIndex_Gthr` table and append all entries our `Vec<SearchEntry>`
pub(crate) fn parse_index_gthr_path(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, HashMap<String, String>>,
    entries: &mut Vec<SearchEntry>,
    evidence: &str,
) -> Result<(), SearchError> {
    for rows in column_rows {
        let mut entry = SearchEntry {
            document_id: 0,
            entry: String::new(),
            last_modified: String::new(),
            properties: HashMap::new(),
            evidence: evidence.to_string(),
        };

        for column in rows {
            match column.column_name.as_str() {
                "DocumentID" => {
                    entry.document_id = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "LastModified" => {
                    let decode_results = base64_decode_standard(&column.column_data);
                    if let Ok(time_data) = decode_results {
                        if time_data.is_empty() {
                            continue;
                        }

                        // Sometimes the last modified data is just ********
                        let asterisk = 0x2a;
                        if time_data[0] == asterisk {
                            continue;
                        }

                        let time_results = nom_unsigned_eight_bytes(&time_data, Endian::Be);
                        if let Ok((_, result)) = time_results {
                            entry.last_modified = filetime_to_iso(result);
                        }
                    }
                }
                "FileName" => entry.entry.clone_from(&column.column_data),
                _ => (),
            }
        }

        if let Some(props) = lookups.get(&entry.document_id.to_string()) {
            entry.properties.clone_from(props);
        }

        entries.push(entry);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_index_gthr, parse_index_gthr_path};
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        artifacts::os::windows::{
            ese::{helper::get_page_data, tables::table_info},
            search::ese::{search_catalog, search_pages},
        },
        filesystem::files::is_file,
        output::manager::OutputManager,
        structs::artifacts::os::windows::SearchOptions,
    };
    use std::{collections::HashMap, path::PathBuf};

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
    fn test_parse_index_gthr() {
        let test_path =
            "C:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb";
        // Some versions of Windows 11 do not use ESE for Windows Search
        if !is_file(test_path) {
            return;
        }
        let mut output = output_options("search_temp", "./tmp", false);

        let catalog = search_catalog(test_path).unwrap();

        let mut gather_table = table_info(&catalog, "SystemIndex_Gthr");
        let gather_pages = search_pages(gather_table.table_page as u32, test_path).unwrap();

        let page_limit = 5;
        let mut gather_chunk = Vec::new();
        let last_page = 0;
        let options = SearchOptions { alt_file: None };

        for gather_page in gather_pages {
            if gather_page == last_page {
                continue;
            }

            gather_chunk.push(gather_page);
            if gather_chunk.len() != page_limit {
                continue;
            }

            let gather_rows = get_page_data(
                test_path,
                &gather_chunk,
                &mut gather_table,
                "SystemIndex_Gthr",
            )
            .unwrap();

            parse_index_gthr(
                &gather_rows.get("SystemIndex_Gthr").unwrap(),
                &HashMap::new(),
                &mut output,
                &options,
                test_path,
            )
            .unwrap();
            break;
        }
    }

    #[test]
    fn test_parse_index_gthr_path() {
        let test_path =
            "C:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb";
        // Some versions of Windows 11 do not use ESE for Windows Search
        if !is_file(test_path) {
            return;
        }

        let catalog = search_catalog(test_path).unwrap();

        let mut gather_table = table_info(&catalog, "SystemIndex_Gthr");
        let gather_pages = search_pages(gather_table.table_page as u32, test_path).unwrap();

        let page_limit = 5;
        let mut gather_chunk = Vec::new();
        let last_page = 0;
        for gather_page in gather_pages {
            if gather_page == last_page {
                continue;
            }

            gather_chunk.push(gather_page);
            if gather_chunk.len() != page_limit {
                continue;
            }

            let gather_rows = get_page_data(
                test_path,
                &gather_chunk,
                &mut gather_table,
                "SystemIndex_Gthr",
            )
            .unwrap();

            let mut entries = Vec::new();

            parse_index_gthr_path(
                &gather_rows.get("SystemIndex_Gthr").unwrap(),
                &HashMap::new(),
                &mut entries,
                test_path,
            )
            .unwrap();
            assert!(entries.len() > 20);
            break;
        }
    }
}
