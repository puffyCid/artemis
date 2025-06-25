use super::{
    error::SearchError,
    properties::parse_prop_id_lookup,
    tables::indexgthr::{parse_index_gthr, parse_index_gthr_path},
};
use crate::{
    artifacts::os::windows::ese::{
        catalog::Catalog,
        helper::{get_all_pages, get_catalog_info, get_filtered_page_data, get_page_data},
        tables::{TableInfo, table_info},
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
    pub(crate) last_modified: String,
    pub(crate) properties: HashMap<String, String>,
}

/// Parse the Windows `Search` ESE database
pub(crate) fn parse_search(
    path: &str,
    output: &mut Output,
    filter: bool,
) -> Result<(), SearchError> {
    let start_time = time_now();
    let catalog = search_catalog(path)?;

    let mut gather_table = table_info(&catalog, "SystemIndex_Gthr");
    let gather_pages = search_pages(gather_table.table_page as u32, path)?;

    let mut property_table = table_info(&catalog, "SystemIndex_PropertyStore");
    let property_pages = search_pages(property_table.table_page as u32, path)?;

    let page_limit = 400;
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

        let mut doc_ids =
            get_document_ids(gather_rows.get("SystemIndex_Gthr").unwrap_or(&Vec::new()));

        let property_rows =
            get_properties(path, &property_pages, &mut property_table, &mut doc_ids);

        let _ = process_search(&property_rows, &gather_rows, output, &start_time, filter);
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

        let mut doc_ids =
            get_document_ids(gather_rows.get("SystemIndex_Gthr").unwrap_or(&Vec::new()));

        let property_rows =
            get_properties(path, &property_pages, &mut property_table, &mut doc_ids);

        let _ = process_search(&property_rows, &gather_rows, output, &start_time, filter);
    }

    Ok(())
}

/// Get `DocumentIDs` from the Search database
pub(crate) fn get_document_ids(entries: &Vec<Vec<TableDump>>) -> HashMap<String, bool> {
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

/// Get properties for the Search database entries
pub(crate) fn get_properties(
    path: &str,
    property_pages: &[u32],
    table: &mut TableInfo,
    doc_ids: &mut HashMap<String, bool>,
) -> HashMap<String, Vec<Vec<TableDump>>> {
    let last_page = 0;
    let page_limit = 300;
    let mut property_chunk = Vec::new();

    let mut property_total_rows =
        HashMap::from([(String::from("SystemIndex_PropertyStore"), Vec::new())]);
    // Not get properties associated with gather entries table
    for property_page in property_pages {
        if property_page == &last_page {
            continue;
        }

        property_chunk.push(*property_page);
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
        let mut property_rows = match rows_results {
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
                property_rows
                    .get_mut("SystemIndex_PropertyStore")
                    .unwrap_or(&mut Vec::new()),
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
        let mut property_rows = match rows_results {
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
                property_rows
                    .get_mut("SystemIndex_PropertyStore")
                    .unwrap_or(&mut Vec::new()),
            );
    }

    property_total_rows
}

/// Process all the Search entries
fn process_search(
    properties: &HashMap<String, Vec<Vec<TableDump>>>,
    gather: &HashMap<String, Vec<Vec<TableDump>>>,
    output: &mut Output,
    start_time: &u64,
    filter: bool,
) -> Result<(), SearchError> {
    let indexes = if let Some(values) = properties.get("SystemIndex_PropertyStore") {
        values
    } else {
        warn!(
            "[search] Could not get table SystemIndex_PropertyStore from ESE results. Something went very wrong"
        );
        return Err(SearchError::MissingIndexes);
    };

    let props = parse_prop_id_lookup(indexes);
    let entries = if let Some(values) = gather.get("SystemIndex_Gthr") {
        values
    } else {
        warn!(
            "[search] Could not get table SystemIndex_Gthr from ESE results. Something went very wrong"
        );
        return Err(SearchError::ParseEse);
    };
    let _ = parse_index_gthr(entries, &props, output, start_time, filter);

    Ok(())
}

/// Get the ESE Catalog
pub(crate) fn search_catalog(path: &str) -> Result<Vec<Catalog>, SearchError> {
    let catalog_result = get_catalog_info(path);
    let catalog = match catalog_result {
        Ok(result) => result,
        Err(err) => {
            error!("[search] Failed to parse {path} catalog: {err:?}");
            return Err(SearchError::ParseEse);
        }
    };

    Ok(catalog)
}

/// Get all pages for the provided table
pub(crate) fn search_pages(table_page: u32, path: &str) -> Result<Vec<u32>, SearchError> {
    let pages_result = get_all_pages(path, table_page);
    let pages = match pages_result {
        Ok(result) => result,
        Err(err) => {
            error!("[search] Failed to get search pages at {path}: {err:?}");
            return Err(SearchError::ParseEse);
        }
    };

    Ok(pages)
}

/// Parse Windows `Search` at provided path
pub(crate) fn parse_search_path(
    path: &str,
    page_limit: &u32,
) -> Result<Vec<SearchEntry>, SearchError> {
    let catalog = search_catalog(path)?;

    let mut gather_table = table_info(&catalog, "SystemIndex_Gthr");
    let gather_pages = search_pages(gather_table.table_page as u32, path)?;

    let mut property_table = table_info(&catalog, "SystemIndex_PropertyStore");
    let property_pages = search_pages(property_table.table_page as u32, path)?;

    let mut gather_chunk = Vec::new();
    let last_page = 0;

    let mut search_entries: Vec<SearchEntry> = Vec::new();

    for gather_page in gather_pages {
        if gather_page == last_page {
            continue;
        }

        gather_chunk.push(gather_page);
        if gather_chunk.len() != (*page_limit as usize) {
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

        let mut doc_ids =
            get_document_ids(gather_rows.get("SystemIndex_Gthr").unwrap_or(&Vec::new()));

        let property_rows =
            get_properties(path, &property_pages, &mut property_table, &mut doc_ids);

        let indexes = if let Some(values) = property_rows.get("SystemIndex_PropertyStore") {
            values
        } else {
            warn!(
                "[search] Could not get table SystemIndex_PropertyStore from ESE results. Something went very wrong"
            );
            return Err(SearchError::MissingIndexes);
        };

        let props = parse_prop_id_lookup(indexes);
        let entries = if let Some(values) = gather_rows.get("SystemIndex_Gthr") {
            values
        } else {
            warn!(
                "[search] Could not get table SystemIndex_Gthr from ESE results. Something went very wrong"
            );
            return Err(SearchError::ParseEse);
        };

        let _ = parse_index_gthr_path(entries, &props, &mut search_entries);
    }

    if !gather_chunk.is_empty() {
        let rows_results =
            get_page_data(path, &gather_chunk, &mut gather_table, "SystemIndex_Gthr");
        let gather_rows = match rows_results {
            Ok(result) => result,
            Err(err) => {
                error!("[search] Failed to parse last SystemIndex_Gthr pages at {path}: {err:?}");
                return Ok(search_entries);
            }
        };

        let mut doc_ids =
            get_document_ids(gather_rows.get("SystemIndex_Gthr").unwrap_or(&Vec::new()));

        let property_rows =
            get_properties(path, &property_pages, &mut property_table, &mut doc_ids);

        let indexes = if let Some(values) = property_rows.get("SystemIndex_PropertyStore") {
            values
        } else {
            warn!(
                "[search] Could not get table SystemIndex_PropertyStore from ESE results. Something went very wrong"
            );
            return Err(SearchError::MissingIndexes);
        };

        let props = parse_prop_id_lookup(indexes);
        let entries = if let Some(values) = gather_rows.get("SystemIndex_Gthr") {
            values
        } else {
            warn!(
                "[search] Could not get table SystemIndex_Gthr from ESE results. Something went very wrong"
            );
            return Err(SearchError::ParseEse);
        };

        let _ = parse_index_gthr_path(entries, &props, &mut search_entries);
    }

    Ok(search_entries)
}

#[cfg(test)]
mod tests {
    use super::{
        get_document_ids, get_properties, parse_search, parse_search_path, process_search,
        search_catalog, search_pages,
    };
    use crate::{
        artifacts::os::windows::ese::{helper::get_page_data, tables::table_info},
        filesystem::files::is_file,
        structs::toml::Output,
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("jsonl"),
            compress,
            timeline: false,
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

        parse_search(test_path, &mut output, false).unwrap();
    }

    #[test]
    fn test_search_catalog() {
        let test_path =
            "C:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb";
        // Some versions of Windows 11 do not use ESE for Windows Search
        if !is_file(test_path) {
            return;
        }

        search_catalog(test_path).unwrap();
    }

    #[test]
    fn test_search_pages() {
        let test_path =
            "C:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb";
        // Some versions of Windows 11 do not use ESE for Windows Search
        if !is_file(test_path) {
            return;
        }

        search_pages(1, test_path).unwrap();
    }

    #[test]
    fn test_get_document_ids() {
        let path = "C:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb";
        // Some versions of Windows 11 do not use ESE for Windows Search
        if !is_file(path) {
            return;
        }

        let catalog = search_catalog(path).unwrap();

        let mut gather_table = table_info(&catalog, "SystemIndex_Gthr");
        let gather_pages = search_pages(gather_table.table_page as u32, path).unwrap();

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

            let gather_rows =
                get_page_data(path, &gather_chunk, &mut gather_table, "SystemIndex_Gthr").unwrap();

            let doc_ids = get_document_ids(
                &gather_rows
                    .get("SystemIndex_Gthr")
                    .unwrap_or(&Vec::new())
                    .to_vec(),
            );

            assert!(!doc_ids.is_empty());

            break;
        }
    }

    #[test]
    fn test_get_properties() {
        let path = "C:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb";
        // Some versions of Windows 11 do not use ESE for Windows Search
        if !is_file(path) {
            return;
        }

        let catalog = search_catalog(path).unwrap();

        let mut gather_table = table_info(&catalog, "SystemIndex_Gthr");
        let gather_pages = search_pages(gather_table.table_page as u32, path).unwrap();

        let mut property_table = table_info(&catalog, "SystemIndex_PropertyStore");
        let property_pages = search_pages(property_table.table_page as u32, path).unwrap();

        let page_limit = 1;
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

            let gather_rows =
                get_page_data(path, &gather_chunk, &mut gather_table, "SystemIndex_Gthr").unwrap();

            let mut doc_ids = get_document_ids(
                &gather_rows
                    .get("SystemIndex_Gthr")
                    .unwrap_or(&Vec::new())
                    .to_vec(),
            );

            let property_rows =
                get_properties(path, &property_pages, &mut property_table, &mut doc_ids);

            assert!(!property_rows.is_empty());

            break;
        }
    }

    #[test]
    fn test_process_search() {
        let path = "C:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb";
        // Some versions of Windows 11 do not use ESE for Windows Search
        if !is_file(path) {
            return;
        }

        let catalog = search_catalog(path).unwrap();

        let mut gather_table = table_info(&catalog, "SystemIndex_Gthr");
        let gather_pages = search_pages(gather_table.table_page as u32, path).unwrap();

        let mut property_table = table_info(&catalog, "SystemIndex_PropertyStore");
        let property_pages = search_pages(property_table.table_page as u32, path).unwrap();

        let page_limit = 5;
        let mut gather_chunk = Vec::new();
        let last_page = 0;

        let mut output = output_options("search_temp", "local", "./tmp", false);

        for gather_page in gather_pages {
            if gather_page == last_page {
                continue;
            }

            gather_chunk.push(gather_page);
            if gather_chunk.len() != page_limit {
                continue;
            }

            let gather_rows =
                get_page_data(path, &gather_chunk, &mut gather_table, "SystemIndex_Gthr").unwrap();

            let mut doc_ids = get_document_ids(
                &gather_rows
                    .get("SystemIndex_Gthr")
                    .unwrap_or(&Vec::new())
                    .to_vec(),
            );

            let property_rows =
                get_properties(path, &property_pages, &mut property_table, &mut doc_ids);

            let _ = process_search(&property_rows, &gather_rows, &mut output, &0, false).unwrap();
            break;
        }
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

        let results = parse_search_path(test_path, &50).unwrap();
        assert!(results.len() > 20);
    }
}
