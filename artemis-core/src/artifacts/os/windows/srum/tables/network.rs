use crate::artifacts::os::windows::srum::error::SrumError;
use common::windows::{NetworkConnectivityInfo, NetworkInfo, TableDump};
use log::error;
use serde_json::Value;
use std::collections::HashMap;

/// Parse the network table from SRUM
pub(crate) fn parse_network(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, String>,
) -> Result<(Value, String), SrumError> {
    let mut network_vec: Vec<NetworkInfo> = Vec::new();
    for rows in column_rows {
        let mut network = NetworkInfo {
            auto_inc_id: 0,
            timestamp: 0,
            app_id: String::new(),
            user_id: String::new(),
            interface_luid: 0,
            l2_profile_id: 0,
            l2_profile_flags: 0,
            bytes_sent: 0,
            bytes_recvd: 0,
        };

        for column in rows {
            match column.column_name.as_str() {
                "AutoIncId" => {
                    network.auto_inc_id = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "TimeStamp" => {
                    network.timestamp = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "AppId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        network.app_id = value.clone();
                        continue;
                    }
                    network.app_id = column.column_data.clone();
                }
                "UserId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        network.user_id = value.clone();
                        continue;
                    }
                    network.user_id = column.column_data.clone();
                }
                "InterfaceLuid" => {
                    network.interface_luid = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "L2ProfileId" => {
                    network.l2_profile_id = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "L2ProfileFlags" => {
                    network.l2_profile_flags =
                        column.column_data.parse::<i32>().unwrap_or_default();
                }
                "BytesSent" => {
                    network.bytes_sent = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "BytesRecvd" => {
                    network.bytes_recvd = column.column_data.parse::<i64>().unwrap_or_default();
                }
                _ => continue,
            }
        }
        network_vec.push(network);
    }

    let serde_data_result = serde_json::to_value(&network_vec);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[srum] Failed to serialize SRUM Network table: {err:?}");
            return Err(SrumError::Serialize);
        }
    };

    Ok((serde_data, String::from("srum_network")))
}

/// Parse the network connectivity table from SRUM
pub(crate) fn parse_network_connectivity(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, String>,
) -> Result<(Value, String), SrumError> {
    let mut network_vec: Vec<NetworkConnectivityInfo> = Vec::new();
    for rows in column_rows {
        let mut network = NetworkConnectivityInfo {
            auto_inc_id: 0,
            timestamp: 0,
            app_id: String::new(),
            user_id: String::new(),
            interface_luid: 0,
            l2_profile_id: 0,
            l2_profile_flags: 0,
            connected_time: 0,
            connect_start_time: 0,
        };

        for column in rows {
            match column.column_name.as_str() {
                "AutoIncId" => {
                    network.auto_inc_id = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "TimeStamp" => {
                    network.timestamp = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "AppId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        network.app_id = value.clone();
                        continue;
                    }
                    network.app_id = column.column_data.clone();
                }
                "UserId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        network.user_id = value.clone();
                        continue;
                    }
                    network.user_id = column.column_data.clone();
                }
                "InterfaceLuid" => {
                    network.interface_luid = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "L2ProfileId" => {
                    network.l2_profile_id = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "L2ProfileFlags" => {
                    network.l2_profile_flags =
                        column.column_data.parse::<i32>().unwrap_or_default();
                }
                "ConnectedTime" => {
                    network.connected_time = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "ConnectStartTime" => {
                    network.connect_start_time =
                        column.column_data.parse::<i64>().unwrap_or_default();
                }
                _ => continue,
            }
        }
        network_vec.push(network);
    }

    let serde_data_result = serde_json::to_value(&network_vec);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[srum] Failed to serialize SRUM Network Connectivity table: {err:?}");
            return Err(SrumError::Serialize);
        }
    };

    Ok((serde_data, String::from("srum_connectivity")))
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{parse_network, parse_network_connectivity};
    use crate::artifacts::os::windows::{
        ese::parser::grab_ese_tables, srum::tables::index::parse_id_lookup,
    };

    #[test]
    fn test_parse_network() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";
        let table = vec![
            String::from("SruDbIdMapTable"),
            String::from("{973F5D5C-1D90-4944-BE8E-24B94231A174}"),
        ];
        let test_data = grab_ese_tables(test_path, &table).unwrap();
        let ids = test_data.get("SruDbIdMapTable").unwrap();
        let id_results = parse_id_lookup(&ids);
        let network = test_data
            .get("{973F5D5C-1D90-4944-BE8E-24B94231A174}")
            .unwrap();

        parse_network(&network, &id_results).unwrap();
    }

    #[test]
    fn test_parse_network_connectivity() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";
        let table = vec![
            String::from("SruDbIdMapTable"),
            String::from("{DD6636C4-8929-4683-974E-22C046A43763}"),
        ];
        let test_data = grab_ese_tables(test_path, &table).unwrap();
        let ids = test_data.get("SruDbIdMapTable").unwrap();
        let id_results = parse_id_lookup(&ids);
        let network = test_data
            .get("{DD6636C4-8929-4683-974E-22C046A43763}")
            .unwrap();

        parse_network_connectivity(&network, &id_results).unwrap();
    }
}
