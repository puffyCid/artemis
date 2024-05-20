use super::{
    error::SearchError,
    properties::parse_prop_id_lookup,
    tables::indexgthr::{parse_index_gthr, parse_index_gthr_path},
};
use crate::{
    artifacts::os::windows::ese::{
        helper::{get_all_pages, get_catalog_info, get_filtered_page_data, get_page_data},
        parser::grab_ese_tables,
        tables::{table_info, TableInfo},
    },
    structs::toml::Output,
    utils::time::time_now,
};
use common::windows::TableDump;
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
    output: &mut Output,
    filter: &bool,
) -> Result<(), SearchError> {
    let start_time = time_now();
    let catalog_result = get_catalog_info(path);
    let catalog = match catalog_result {
        Ok(result) => result,
        Err(err) => {
            error!("[search] Failed to parse {path} catalog: {err:?}");
            return Err(SearchError::ParseEse);
        }
    };

    let mut gather_table = table_info(&catalog, "SystemIndex_Gthr");
    let pages_result = get_all_pages(path, &(gather_table.table_page as u32));
    let gather_pages = match pages_result {
        Ok(result) => result,
        Err(err) => {
            error!("[search] Failed to get SystemIndex_Gthr pages at {path}: {err:?}");
            return Err(SearchError::ParseEse);
        }
    };

    let mut property_table = table_info(&catalog, "SystemIndex_PropertyStore");
    let pages_result = get_all_pages(path, &(property_table.table_page as u32));
    let property_pages = match pages_result {
        Ok(result) => result,
        Err(err) => {
            error!("[search] Failed to get SystemIndex_PropertyStore pages at {path}: {err:?}");
            return Err(SearchError::ParseEse);
        }
    };

    let page_limit = 30;
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

        let rows_results =
            get_page_data(path, &gather_chunk, &mut gather_table, "SystemIndex_Gthr");
        let gather_rows = match rows_results {
            Ok(result) => result,
            Err(err) => {
                error!("[search] Failed to parse SystemIndex_Gthr table at {path}: {err:?}");
                continue;
            }
        };

        let mut doc_ids = get_document_ids(
            &gather_rows
                .get("SystemIndex_Gthr")
                .unwrap_or(&Vec::new())
                .to_vec(),
        );

        let property_rows =
            get_properties(path, &property_pages, &mut property_table, &mut doc_ids);

        process_search(&property_rows, &gather_rows, output, &start_time, filter);
        gather_chunk = Vec::new();
    }

    if !gather_chunk.is_empty() {
        let rows_results =
            get_page_data(path, &gather_chunk, &mut gather_table, "SystemIndex_Gthr");
        let gather_rows = match rows_results {
            Ok(result) => result,
            Err(err) => {
                error!("[search] Failed to parse last SystemIndex_Gthr pages at {path}: {err:?}");
                return Ok(());
            }
        };

        let mut doc_ids = get_document_ids(
            &gather_rows
                .get("SystemIndex_Gthr")
                .unwrap_or(&Vec::new())
                .to_vec(),
        );

        let property_rows =
            get_properties(path, &property_pages, &mut property_table, &mut doc_ids);

        process_search(&property_rows, &gather_rows, output, &start_time, filter);
    }

    Ok(())
}

fn get_document_ids(entries: &Vec<Vec<TableDump>>) -> HashMap<String, bool> {
    let mut values = HashMap::new();
    for row in entries {
        for column in row {
            if column.column_name != "DocumentID" {
                continue;
            }

            values.insert(column.column_data.clone(), true);
        }
    }

    values
}

fn get_properties(
    path: &str,
    property_pages: &[u32],
    table: &mut TableInfo,
    doc_ids: &mut HashMap<String, bool>,
) -> HashMap<String, Vec<Vec<TableDump>>> {
    let last_page = 0;
    let page_limit = 80;
    let mut property_chunk = Vec::new();

    let mut property_total_rows =
        HashMap::from([(String::from("SystemIndex_PropertyStore"), Vec::new())]);
    // Not get properties associated with gather entries table
    for property_page in property_pages {
        if property_page == &last_page {
            continue;
        }

        property_chunk.push(property_page.clone());
        if property_chunk.len() != page_limit {
            continue;
        }

        let rows_results = get_filtered_page_data(
            path,
            &property_chunk,
            table,
            "SystemIndex_PropertyStore",
            "WorkID",
            doc_ids,
        );
        let property_rows = match rows_results {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "[search] Failed to parse SystemIndex_PropertyStore table at {path}: {err:?}"
                );
                continue;
            }
        };

        property_total_rows
            .entry(String::from("SystemIndex_PropertyStore"))
            .or_insert(Vec::new())
            .append(
                &mut property_rows
                    .get("SystemIndex_PropertyStore")
                    .unwrap_or(&Vec::new())
                    .to_vec(),
            );

        if doc_ids.is_empty() {
            break;
        }

        property_chunk = Vec::new();
    }

    if !property_chunk.is_empty() {
        let rows_results = get_filtered_page_data(
            path,
            &property_chunk,
            table,
            "SystemIndex_PropertyStore",
            "WorkID",
            doc_ids,
        );
        let property_rows = match rows_results {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "[search] Failed to parse last SystemIndex_PropertyStore page at {path}: {err:?}"
                );
                return property_total_rows;
            }
        };

        property_total_rows
            .entry(String::from("SystemIndex_PropertyStore"))
            .or_insert(Vec::new())
            .append(
                &mut property_rows
                    .get("SystemIndex_PropertyStore")
                    .unwrap_or(&Vec::new())
                    .to_vec(),
            );
    }

    property_total_rows
}

fn process_search(
    properties: &HashMap<String, Vec<Vec<TableDump>>>,
    gather: &HashMap<String, Vec<Vec<TableDump>>>,
    output: &mut Output,
    start_time: &u64,
    filter: &bool,
) -> Result<(), SearchError> {
    let indexes = if let Some(values) = properties.get("SystemIndex_PropertyStore") {
        values
    } else {
        warn!("[search] Could not get table SystemIndex_PropertyStore from ESE results. Something went very wrong");
        return Err(SearchError::MissingIndexes);
    };

    let props = parse_prop_id_lookup(indexes);
    let entries = if let Some(values) = gather.get("SystemIndex_Gthr") {
        values
    } else {
        warn!("[search] Could not get table SystemIndex_Gthr from ESE results. Something went very wrong");
        return Err(SearchError::ParseEse);
    };
    let _ = parse_index_gthr(entries, &props, output, &start_time, filter);

    Ok(())
}

/// Parse Windows `Search` at provided path
pub(crate) fn parse_search_path(
    path: &str,
    tables: &[String],
) -> Result<Vec<SearchEntry>, SearchError> {
    let table_results = grab_ese_tables(path, tables);
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
    use crate::{filesystem::files::is_file, structs::toml::Output};

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
    #[ignore = "Can take a long time"]
    fn test_parse_search() {
        let test_path =
            "C:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb";
        // Some versions of Windows 11 do not use ESE for Windows Search
        if !is_file(test_path) {
            return;
        }
        let mut output = output_options("search_temp", "local", "./tmp", false);

        parse_search(test_path, &mut output, &false).unwrap();
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
