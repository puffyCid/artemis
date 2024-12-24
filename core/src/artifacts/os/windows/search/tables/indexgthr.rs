use crate::{
    artifacts::os::windows::{
        artifacts::output_data,
        search::{error::SearchError, ese::SearchEntry},
    },
    structs::toml::Output,
    utils::{
        encoding::base64_decode_standard,
        nom_helper::{nom_unsigned_eight_bytes, Endian},
        time::{filetime_to_unixepoch, unixepoch_to_iso},
    },
};
use common::windows::TableDump;
use log::error;
use std::collections::HashMap;

/// Parse the `SystemIndex_Gthr` table and output data
pub(crate) fn parse_index_gthr(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, HashMap<String, String>>,
    output: &mut Output,
    start_time: &u64,
    filter: &bool,
) -> Result<(), SearchError> {
    let mut entries = Vec::new();
    let limit = 100000;

    for rows in column_rows {
        let mut entry = SearchEntry {
            document_id: 0,
            entry: String::new(),
            last_modified: String::from("1970-01-01T00:00:00.000Z"),
            properties: HashMap::new(),
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
                            entry.last_modified = unixepoch_to_iso(&filetime_to_unixepoch(&result));
                        }
                    }
                }
                "FileName" => entry.entry.clone_from(&column.column_data),
                _ => continue,
            }
        }

        if let Some(props) = lookups.get(&entry.document_id.to_string()) {
            entry.properties.clone_from(props);
        }

        entries.push(entry);

        // We set a limit just in case a system has indexed a lot of data
        if entries.len() == limit {
            let serde_data_result = serde_json::to_value(&entries);
            let mut serde_data = match serde_data_result {
                Ok(results) => results,
                Err(err) => {
                    error!("[search] Failed to serialize Index Gthr table: {err:?}");
                    return Err(SearchError::Serialize);
                }
            };
            let result = output_data(&mut serde_data, "search", output, start_time, filter);
            match result {
                Ok(_result) => {}
                Err(err) => {
                    error!("[search] Could not output Index Gthr search data: {err:?}");
                }
            }

            entries = Vec::new();
        }
    }

    if entries.is_empty() {
        return Ok(());
    }

    // Output any leftover data
    let serde_data_result = serde_json::to_value(&entries);
    let mut serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[search] Failed to serialize Index Gthr table: {err:?}");
            return Err(SearchError::Serialize);
        }
    };
    let result = output_data(&mut serde_data, "search", output, start_time, filter);
    match result {
        Ok(_result) => {}
        Err(err) => {
            error!("[search] Could not output Index Gthr search data: {err:?}");
        }
    }

    Ok(())
}

/// Parse the `SystemIndex_Gthr` table and append all entries our `Vec<SearchEntry>`
pub(crate) fn parse_index_gthr_path(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, HashMap<String, String>>,
    entries: &mut Vec<SearchEntry>,
) -> Result<(), SearchError> {
    for rows in column_rows {
        let mut entry = SearchEntry {
            document_id: 0,
            entry: String::new(),
            last_modified: String::new(),
            properties: HashMap::new(),
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
                            entry.last_modified = unixepoch_to_iso(&filetime_to_unixepoch(&result));
                        }
                    }
                }
                "FileName" => entry.entry.clone_from(&column.column_data),
                _ => continue,
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
    use crate::{
        artifacts::os::windows::{
            ese::{helper::get_page_data, tables::table_info},
            search::ese::{search_catalog, search_pages},
        },
        filesystem::files::is_file,
        structs::toml::Output,
    };
    use std::collections::HashMap;

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
    fn test_parse_index_gthr() {
        let test_path =
            "C:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb";
        // Some versions of Windows 11 do not use ESE for Windows Search
        if !is_file(test_path) {
            return;
        }
        let mut output = output_options("search_temp", "local", "./tmp", false);

        let catalog = search_catalog(test_path).unwrap();

        let mut gather_table = table_info(&catalog, "SystemIndex_Gthr");
        let gather_pages = search_pages(&(gather_table.table_page as u32), test_path).unwrap();

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

            parse_index_gthr(
                &gather_rows.get("SystemIndex_Gthr").unwrap(),
                &HashMap::new(),
                &mut output,
                &0,
                &false,
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
        let gather_pages = search_pages(&(gather_table.table_page as u32), test_path).unwrap();

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
            )
            .unwrap();
            assert!(entries.len() > 20);
            break;
        }
    }
}
