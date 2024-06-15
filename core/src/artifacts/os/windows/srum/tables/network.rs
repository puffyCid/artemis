use crate::{
    artifacts::os::windows::srum::error::SrumError,
    utils::time::{filetime_to_unixepoch, unixepoch_to_iso},
};
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
            timestamp: String::new(),
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
                    network.timestamp.clone_from(&column.column_data);
                    // unixepoch_to_iso(&column.column_data.parse::<i64>().unwrap_or_default());
                }
                "AppId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        network.app_id.clone_from(value);
                        continue;
                    }
                    network.app_id.clone_from(&column.column_data);
                }
                "UserId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        network.user_id.clone_from(value);
                        continue;
                    }
                    network.user_id.clone_from(&column.column_data);
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
            timestamp: String::new(),
            app_id: String::new(),
            user_id: String::new(),
            interface_luid: 0,
            l2_profile_id: 0,
            l2_profile_flags: 0,
            connected_time: 0,
            connect_start_time: String::new(),
        };

        for column in rows {
            match column.column_name.as_str() {
                "AutoIncId" => {
                    network.auto_inc_id = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "TimeStamp" => {
                    network.timestamp.clone_from(&column.column_data);
                    //unixepoch_to_iso(&column.column_data.parse::<i64>().unwrap_or_default());
                }
                "AppId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        network.app_id.clone_from(value);
                        continue;
                    }
                    network.app_id.clone_from(&column.column_data);
                }
                "UserId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        network.user_id.clone_from(value);
                        continue;
                    }
                    network.user_id.clone_from(&column.column_data);
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
                    network.connect_start_time = unixepoch_to_iso(&filetime_to_unixepoch(
                        &column.column_data.parse::<u64>().unwrap_or_default(),
                    ));
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
    use crate::artifacts::os::windows::srum::{
        resource::get_srum_ese, tables::index::parse_id_lookup,
    };

    #[test]
    fn test_parse_network() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";

        let indexes = get_srum_ese(test_path, "SruDbIdMapTable").unwrap();
        let lookups = parse_id_lookup(&indexes);
        let srum_data = get_srum_ese(test_path, "{973F5D5C-1D90-4944-BE8E-24B94231A174}").unwrap();

        parse_network(&srum_data, &lookups).unwrap();
    }

    #[test]
    fn test_parse_network_connectivity() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";

        let indexes = get_srum_ese(test_path, "SruDbIdMapTable").unwrap();
        let lookups = parse_id_lookup(&indexes);
        let srum_data = get_srum_ese(test_path, "{DD6636C4-8929-4683-974E-22C046A43763}").unwrap();

        parse_network_connectivity(&srum_data, &lookups).unwrap();
    }
}
