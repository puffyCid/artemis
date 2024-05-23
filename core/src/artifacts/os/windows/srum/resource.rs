use super::{
    error::SrumError,
    tables::{
        application::{parse_app_timeline, parse_application, parse_vfu_provider},
        energy::{parse_energy, parse_energy_usage},
        network::{parse_network, parse_network_connectivity},
        notifications::parse_notification,
    },
};
use crate::{
    artifacts::os::windows::{
        artifacts::output_data,
        ese::{
            helper::{get_all_pages, get_catalog_info, get_page_data},
            tables::table_info,
        },
        srum::tables::index::parse_id_lookup,
    },
    structs::toml::Output,
    utils::time::time_now,
};
use common::windows::TableDump;
use log::{error, warn};
use serde_json::Value;

/// Parse and dump the provided SRUM tables
pub(crate) fn parse_srum(path: &str, output: &mut Output, filter: &bool) -> Result<(), SrumError> {
    let start_time = time_now();

    let indexes = get_srum_ese(path, "SruDbIdMapTable")?;
    let lookups = parse_id_lookup(&indexes);

    let tables = vec![
        "{5C8CF1C7-7257-4F13-B223-970EF5939312}",
        "{973F5D5C-1D90-4944-BE8E-24B94231A174}",
        "{7ACBBAA3-D029-4BE4-9A7A-0885927F1D8F}",
        "{D10CA2FE-6FCF-4F6D-848E-B2E99266FA86}",
        "{D10CA2FE-6FCF-4F6D-848E-B2E99266FA89}",
        "{DA73FB89-2BEA-4DDC-86B8-6E048C6DA477}",
        "{DD6636C4-8929-4683-974E-22C046A43763}",
        "{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}",
        "{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}LT",
    ];

    for table in tables {
        let srum_data = get_srum_ese(path, table)?;

        let (serde_data, table_type) = match table {
            "{5C8CF1C7-7257-4F13-B223-970EF5939312}" => parse_app_timeline(&srum_data, &lookups)?,
            "{973F5D5C-1D90-4944-BE8E-24B94231A174}" => parse_network(&srum_data, &lookups)?,
            "{DD6636C4-8929-4683-974E-22C046A43763}" => {
                parse_network_connectivity(&srum_data, &lookups)?
            }
            "{D10CA2FE-6FCF-4F6D-848E-B2E99266FA86}" => parse_notification(&srum_data, &lookups)?,
            "{D10CA2FE-6FCF-4F6D-848E-B2E99266FA89}" => parse_application(&srum_data, &lookups)?,
            "{DA73FB89-2BEA-4DDC-86B8-6E048C6DA477}" => parse_energy(&srum_data, &lookups)?,
            "{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}"
            | "{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}LT" => {
                parse_energy_usage(&srum_data, &lookups)?
            }
            "{7ACBBAA3-D029-4BE4-9A7A-0885927F1D8F}" => parse_vfu_provider(&srum_data, &lookups)?,
            _ => continue,
        };

        let result = output_data(&serde_data, &table_type, output, &start_time, filter);
        match result {
            Ok(_result) => {}
            Err(err) => {
                error!("[srum] Could not output {table_type} data: {err:?}");
            }
        }
    }

    Ok(())
}

/// Get single SRUM table
pub(crate) fn get_srum(path: &str, table: &str) -> Result<Value, SrumError> {
    let indexes = get_srum_ese(path, "SruDbIdMapTable")?;
    let lookups = parse_id_lookup(&indexes);
    let srum_data = get_srum_ese(path, table)?;

    let (srum_data, _table_type) = match table {
        "{5C8CF1C7-7257-4F13-B223-970EF5939312}" => parse_app_timeline(&srum_data, &lookups)?,
        "{973F5D5C-1D90-4944-BE8E-24B94231A174}" => parse_network(&srum_data, &lookups)?,
        "{DD6636C4-8929-4683-974E-22C046A43763}" => {
            parse_network_connectivity(&srum_data, &lookups)?
        }
        "{D10CA2FE-6FCF-4F6D-848E-B2E99266FA86}" => parse_notification(&srum_data, &lookups)?,
        "{D10CA2FE-6FCF-4F6D-848E-B2E99266FA89}" => parse_application(&srum_data, &lookups)?,
        "{DA73FB89-2BEA-4DDC-86B8-6E048C6DA477}" => parse_energy(&srum_data, &lookups)?,
        "{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}" | "{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}LT" => {
            parse_energy_usage(&srum_data, &lookups)?
        }
        "{7ACBBAA3-D029-4BE4-9A7A-0885927F1D8F}" => parse_vfu_provider(&srum_data, &lookups)?,
        _ => {
            return Err(SrumError::NoTable);
        }
    };
    Ok(srum_data)
}

/// Extract SRUM info from ESE database
pub(crate) fn get_srum_ese(path: &str, table: &str) -> Result<Vec<Vec<TableDump>>, SrumError> {
    let catalog_result = get_catalog_info(path);
    let catalog = match catalog_result {
        Ok(result) => result,
        Err(err) => {
            error!("[srum] Failed to parse {path} catalog: {err:?}");
            return Err(SrumError::ParseEse);
        }
    };

    let mut info = table_info(&catalog, table);
    if info.table_name.is_empty() || info.table_page == 0 {
        warn!("[srum] No hit for table: {table}");
        return Ok(Vec::new());
    }
    let pages_result = get_all_pages(path, &(info.table_page as u32));
    let pages = match pages_result {
        Ok(result) => result,
        Err(err) => {
            error!("[srum] Failed to get {table} pages at {path}: {err:?}");
            return Err(SrumError::ParseEse);
        }
    };

    let rows_results = get_page_data(path, &pages, &mut info, table);
    let table_rows = match rows_results {
        Ok(result) => result,
        Err(err) => {
            error!("[srum] Failed to parse {table} table at {path}: {err:?}");
            return Err(SrumError::ParseEse);
        }
    };

    Ok(table_rows.get(table).unwrap_or(&Vec::new()).clone())
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{get_srum, get_srum_ese, parse_srum};
    use crate::structs::toml::Output;

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
    fn test_parse_srum() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";
        let mut output = output_options("srum_temp", "local", "./tmp", true);

        parse_srum(test_path, &mut output, &false).unwrap();
    }

    #[test]
    fn test_get_srum_ese() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";

        get_srum_ese(test_path, "{5C8CF1C7-7257-4F13-B223-970EF5939312}").unwrap();
    }

    #[test]
    fn test_get_srum() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";

        let results = get_srum(test_path, "{5C8CF1C7-7257-4F13-B223-970EF5939312}").unwrap();
        assert_eq!(results.is_null(), false)
    }
}
