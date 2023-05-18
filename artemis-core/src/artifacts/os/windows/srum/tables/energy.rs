use crate::artifacts::os::windows::{ese::parser::TableDump, srum::error::SrumError};
use log::error;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
struct EnergyInfo {
    auto_inc_id: i32,
    timestamp: i64,
    app_id: String,
    user_id: String,
    binary_data: String,
}

#[derive(Debug, Serialize)]
struct EnergyUsage {
    auto_inc_id: i32,
    timestamp: i64,
    app_id: String,
    user_id: String,
    event_timestamp: i64,
    state_transition: i32,
    full_charged_capacity: i32,
    designed_capacity: i32,
    charge_level: i32,
    cycle_count: i32,
    configuration_hash: i64,
}

/// Parse the unknown energy table from SRUM
pub(crate) fn parse_energy(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, String>,
) -> Result<(Value, String), SrumError> {
    let mut energy_vec: Vec<EnergyInfo> = Vec::new();
    for rows in column_rows {
        let mut energy = EnergyInfo {
            auto_inc_id: 0,
            timestamp: 0,
            app_id: String::new(),
            user_id: String::new(),
            binary_data: String::new(),
        };

        for column in rows {
            match column.column_name.as_str() {
                "AutoIncId" => {
                    energy.auto_inc_id = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "TimeStamp" => {
                    energy.timestamp = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "AppId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        energy.app_id = value.clone();
                        continue;
                    }
                    energy.app_id = column.column_data.clone();
                }
                "UserId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        energy.user_id = value.clone();
                        continue;
                    }
                    energy.user_id = column.column_data.clone();
                }
                "BinaryData" => energy.binary_data = column.column_data.clone(),
                _ => continue,
            }
        }
        energy_vec.push(energy);
    }

    let serde_data_result = serde_json::to_value(&energy_vec);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[srum] Failed to serialize SRUM Unknown Energy table: {err:?}");
            return Err(SrumError::Serialize);
        }
    };

    Ok((serde_data, String::from("srum_energy")))
}

/// Parse the energy usage table from SRUM
pub(crate) fn parse_energy_usage(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, String>,
) -> Result<(Value, String), SrumError> {
    let mut energy_vec: Vec<EnergyUsage> = Vec::new();
    for rows in column_rows {
        let mut energy = EnergyUsage {
            auto_inc_id: 0,
            timestamp: 0,
            app_id: String::new(),
            user_id: String::new(),
            event_timestamp: 0,
            state_transition: 0,
            full_charged_capacity: 0,
            designed_capacity: 0,
            charge_level: 0,
            cycle_count: 0,
            configuration_hash: 0,
        };

        for column in rows {
            match column.column_name.as_str() {
                "AutoIncId" => energy.auto_inc_id = column.column_data.parse::<i32>().unwrap(),
                "TimeStamp" => {
                    energy.timestamp = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "AppId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        energy.app_id = value.clone();
                        continue;
                    }
                    energy.app_id = column.column_data.clone();
                }
                "UserId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        energy.user_id = value.clone();
                        continue;
                    }
                    energy.user_id = column.column_data.clone();
                }
                "EventTimestamp" => {
                    energy.event_timestamp = column.column_data.parse::<i64>().unwrap();
                }
                "StateTransition" => {
                    energy.state_transition = column.column_data.parse::<i32>().unwrap();
                }
                "DesignedCapacity" => {
                    energy.designed_capacity = column.column_data.parse::<i32>().unwrap();
                }
                "FullChargedCapacity" => {
                    energy.full_charged_capacity = column.column_data.parse::<i32>().unwrap();
                }
                "ChargeLevel" => energy.charge_level = column.column_data.parse::<i32>().unwrap(),
                "CycleCount" => energy.cycle_count = column.column_data.parse::<i32>().unwrap(),
                "ConfigurationHash" => {
                    energy.configuration_hash = column.column_data.parse::<i64>().unwrap();
                }

                _ => continue,
            }
        }
        energy_vec.push(energy);
    }

    let serde_data_result = serde_json::to_value(&energy_vec);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[srum] Failed to serialize SRUM Energy Usage table: {err:?}");
            return Err(SrumError::Serialize);
        }
    };

    Ok((serde_data, String::from("srum_energy_usage")))
}

#[cfg(test)]
mod tests {
    use super::{parse_energy, parse_energy_usage};
    use crate::artifacts::os::windows::{
        ese::parser::grab_ese_tables_path, srum::tables::index::parse_id_lookup,
    };

    #[test]
    fn test_parse_energy() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";
        let table = vec![
            String::from("SruDbIdMapTable"),
            String::from("{DA73FB89-2BEA-4DDC-86B8-6E048C6DA477}"),
        ];
        let test_data = grab_ese_tables_path(test_path, &table).unwrap();
        let ids = test_data.get("SruDbIdMapTable").unwrap();
        let id_results = parse_id_lookup(&ids);
        let energy = test_data
            .get("{DA73FB89-2BEA-4DDC-86B8-6E048C6DA477}")
            .unwrap();

        parse_energy(&energy, &id_results).unwrap();
    }

    #[test]
    fn test_parse_energy_usage() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";
        let table = vec![
            String::from("SruDbIdMapTable"),
            String::from("{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}"),
        ];
        let test_data = grab_ese_tables_path(test_path, &table).unwrap();
        let ids = test_data.get("SruDbIdMapTable").unwrap();
        let id_results = parse_id_lookup(&ids);
        let energy = test_data
            .get("{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}")
            .unwrap();

        parse_energy_usage(&energy, &id_results).unwrap();
    }

    #[test]
    fn test_parse_energy_usagelt() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";
        let table = vec![
            String::from("SruDbIdMapTable"),
            String::from("{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}LT"),
        ];
        let test_data = grab_ese_tables_path(test_path, &table).unwrap();
        let ids = test_data.get("SruDbIdMapTable").unwrap();
        let id_results = parse_id_lookup(&ids);
        let energy = test_data
            .get("{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}LT")
            .unwrap();

        parse_energy_usage(&energy, &id_results).unwrap();
    }
}
