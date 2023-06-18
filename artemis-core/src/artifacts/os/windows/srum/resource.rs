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
        artifacts::output_data, ese::parser::grab_ese_tables_path,
        srum::tables::index::parse_id_lookup,
    },
    utils::{artemis_toml::Output, time::time_now},
};
use log::{error, warn};
use serde_json::Value;

/// Parse and dump the provided SRUM tables
pub(crate) fn parse_srum(
    path: &str,
    tables: &[String],
    output: &mut Output,
    filter: &bool,
) -> Result<(), SrumError> {
    let start_time = time_now();

    let table_results = grab_ese_tables_path(path, tables);
    let table_data = match table_results {
        Ok(results) => results,
        Err(err) => {
            error!("[srum] Failed to parse {path} ESE file: {err:?}");
            return Err(SrumError::ParseEse);
        }
    };

    let indexes = if let Some(values) = table_data.get("SruDbIdMapTable") {
        values
    } else {
        warn!("[srum] Could not get table SruDbIdMapTable from ESE results");
        return Err(SrumError::MissingIndexes);
    };
    let lookups = parse_id_lookup(indexes);

    for table in tables {
        let srum_data = if let Some(values) = table_data.get(table) {
            values
        } else {
            warn!("[srum] Could not get table {table} from ESE results");
            continue;
        };

        let (serde_data, table_type) = match table.as_str() {
            "{5C8CF1C7-7257-4F13-B223-970EF5939312}" => parse_app_timeline(srum_data, &lookups)?,
            "{973F5D5C-1D90-4944-BE8E-24B94231A174}" => parse_network(srum_data, &lookups)?,
            "{DD6636C4-8929-4683-974E-22C046A43763}" => {
                parse_network_connectivity(srum_data, &lookups)?
            }
            "{D10CA2FE-6FCF-4F6D-848E-B2E99266FA86}" => parse_notification(srum_data, &lookups)?,
            "{D10CA2FE-6FCF-4F6D-848E-B2E99266FA89}" => parse_application(srum_data, &lookups)?,
            "{DA73FB89-2BEA-4DDC-86B8-6E048C6DA477}" => parse_energy(srum_data, &lookups)?,
            "{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}"
            | "{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}LT" => {
                parse_energy_usage(srum_data, &lookups)?
            }
            "{7ACBBAA3-D029-4BE4-9A7A-0885927F1D8F}" => parse_vfu_provider(srum_data, &lookups)?,
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

pub(crate) fn get_srum(path: &str, tables: &[String]) -> Result<Value, SrumError> {
    let table_results = grab_ese_tables_path(path, tables);
    let table_data = match table_results {
        Ok(results) => results,
        Err(err) => {
            error!("[srum] Failed to parse {path} ESE file: {err:?}");
            return Err(SrumError::ParseEse);
        }
    };

    let indexes = if let Some(values) = table_data.get("SruDbIdMapTable") {
        values
    } else {
        warn!("[srum] Could not get table SruDbIdMapTable from ESE results");
        return Err(SrumError::MissingIndexes);
    };
    let lookups = parse_id_lookup(indexes);

    for table in tables {
        let srum_data = if let Some(values) = table_data.get(table) {
            values
        } else {
            warn!("[srum] Could not get table {table} from ESE results");
            continue;
        };

        let (srum_data, _table_type) = match table.as_str() {
            "{5C8CF1C7-7257-4F13-B223-970EF5939312}" => parse_app_timeline(srum_data, &lookups)?,
            "{973F5D5C-1D90-4944-BE8E-24B94231A174}" => parse_network(srum_data, &lookups)?,
            "{DD6636C4-8929-4683-974E-22C046A43763}" => {
                parse_network_connectivity(srum_data, &lookups)?
            }
            "{D10CA2FE-6FCF-4F6D-848E-B2E99266FA86}" => parse_notification(srum_data, &lookups)?,
            "{D10CA2FE-6FCF-4F6D-848E-B2E99266FA89}" => parse_application(srum_data, &lookups)?,
            "{DA73FB89-2BEA-4DDC-86B8-6E048C6DA477}" => parse_energy(srum_data, &lookups)?,
            "{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}"
            | "{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}LT" => {
                parse_energy_usage(srum_data, &lookups)?
            }
            "{7ACBBAA3-D029-4BE4-9A7A-0885927F1D8F}" => parse_vfu_provider(srum_data, &lookups)?,
            _ => {
                continue;
            }
        };
        return Ok(srum_data);
    }
    Err(SrumError::NoTable)
}

#[cfg(test)]
mod tests {
    use super::{get_srum, parse_srum};
    use crate::utils::artemis_toml::Output;

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
        }
    }

    #[test]
    fn test_parse_srum() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";
        let tables = vec![
            String::from("{5C8CF1C7-7257-4F13-B223-970EF5939312}"),
            String::from("{973F5D5C-1D90-4944-BE8E-24B94231A174}"),
            String::from("{7ACBBAA3-D029-4BE4-9A7A-0885927F1D8F}"),
            String::from("{D10CA2FE-6FCF-4F6D-848E-B2E99266FA86}"),
            String::from("{D10CA2FE-6FCF-4F6D-848E-B2E99266FA89}"),
            String::from("{DA73FB89-2BEA-4DDC-86B8-6E048C6DA477}"),
            String::from("{DD6636C4-8929-4683-974E-22C046A43763}"),
            String::from("{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}"),
            String::from("{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}LT"),
            String::from("SruDbIdMapTable"),
        ];
        let mut output = output_options("srum_temp", "local", "./tmp", false);

        parse_srum(test_path, &tables, &mut output, &false).unwrap();
    }

    #[test]
    fn test_get_srum() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";
        let tables = vec![
            String::from("{5C8CF1C7-7257-4F13-B223-970EF5939312}"),
            String::from("SruDbIdMapTable"),
        ];

        let results = get_srum(test_path, &tables).unwrap();
        assert_eq!(results.is_null(), false)
    }
}
