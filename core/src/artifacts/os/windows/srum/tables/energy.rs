use crate::{
    artifacts::os::windows::srum::error::SrumError,
    utils::time::{filetime_to_unixepoch, unixepoch_to_iso},
};
use common::windows::{EnergyInfo, EnergyUsage, TableDump};
use log::error;
use serde_json::Value;
use std::collections::HashMap;

/// Parse the unknown energy table from SRUM
pub(crate) fn parse_energy(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, String>,
) -> Result<(Value, String), SrumError> {
    let mut energy_vec: Vec<EnergyInfo> = Vec::new();
    for rows in column_rows {
        let mut energy = EnergyInfo {
            auto_inc_id: 0,
            timestamp: String::new(),
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
                    energy.timestamp.clone_from(&column.column_data);
                }
                "AppId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        energy.app_id.clone_from(value);
                        continue;
                    }
                    energy.app_id.clone_from(&column.column_data);
                }
                "UserId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        energy.user_id.clone_from(value);
                        continue;
                    }
                    energy.user_id.clone_from(&column.column_data);
                }
                "BinaryData" => energy.binary_data.clone_from(&column.column_data),
                _ => (),
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
            timestamp: String::new(),
            app_id: String::new(),
            user_id: String::new(),
            event_timestamp: String::new(),
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
                    energy.timestamp.clone_from(&column.column_data);
                }
                "AppId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        energy.app_id.clone_from(value);
                        continue;
                    }
                    energy.app_id.clone_from(&column.column_data);
                }
                "UserId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        energy.user_id.clone_from(value);
                        continue;
                    }
                    energy.user_id.clone_from(&column.column_data);
                }
                "EventTimestamp" => {
                    energy.event_timestamp = unixepoch_to_iso(&filetime_to_unixepoch(
                        &column.column_data.parse::<u64>().unwrap_or_default(),
                    ));
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

                _ => (),
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
#[cfg(target_os = "windows")]
mod tests {
    use super::{parse_energy, parse_energy_usage};
    use crate::artifacts::os::windows::srum::{
        resource::get_srum_ese, tables::index::parse_id_lookup,
    };

    #[test]
    fn test_parse_energy() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";

        let indexes = get_srum_ese(test_path, "SruDbIdMapTable").unwrap();
        let lookups = parse_id_lookup(&indexes);
        let energy_check = get_srum_ese(test_path, "{DA73FB89-2BEA-4DDC-86B8-6E048C6DA477}");
        if energy_check.is_err() {
            return;
        }

        parse_energy(&energy_check.unwrap(), &lookups).unwrap();
    }

    #[test]
    fn test_parse_energy_usage() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";

        let indexes = get_srum_ese(test_path, "SruDbIdMapTable").unwrap();
        let lookups = parse_id_lookup(&indexes);
        let srum_data = get_srum_ese(test_path, "{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}").unwrap();

        parse_energy_usage(&srum_data, &lookups).unwrap();
    }

    #[test]
    fn test_parse_energy_usagelt() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";

        let indexes = get_srum_ese(test_path, "SruDbIdMapTable").unwrap();
        let lookups = parse_id_lookup(&indexes);
        let srum_data =
            get_srum_ese(test_path, "{FEE4E14F-02A9-4550-B5CE-5FA2DA202E37}LT").unwrap();

        parse_energy_usage(&srum_data, &lookups).unwrap();
    }
}
