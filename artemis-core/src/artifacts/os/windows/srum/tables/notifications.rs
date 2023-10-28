use crate::artifacts::os::windows::{ese::parser::TableDump, srum::error::SrumError};
use log::error;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Serialize)]
struct NotificationInfo {
    auto_inc_id: i32,
    timestamp: i64,
    app_id: String,
    user_id: String,
    notification_type: i32,
    payload_size: i32,
    network_type: i32,
}

/// Parse the notification table from SRUM
pub(crate) fn parse_notification(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, String>,
) -> Result<(Value, String), SrumError> {
    let mut notif_vec: Vec<NotificationInfo> = Vec::new();
    for rows in column_rows {
        let mut notif = NotificationInfo {
            auto_inc_id: 0,
            timestamp: 0,
            app_id: String::new(),
            user_id: String::new(),
            notification_type: 0,
            payload_size: 0,
            network_type: 0,
        };

        for column in rows {
            match column.column_name.as_str() {
                "AutoIncId" => {
                    notif.auto_inc_id = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "TimeStamp" => {
                    notif.timestamp = column.column_data.parse::<i64>().unwrap_or_default();
                }
                "AppId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        notif.app_id = value.clone();
                        continue;
                    }
                    notif.app_id = column.column_data.clone();
                }
                "UserId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        notif.user_id = value.clone();
                        continue;
                    }
                    notif.user_id = column.column_data.clone();
                }
                "NotificationType" => {
                    notif.notification_type = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "PayloadSize" => {
                    notif.payload_size = column.column_data.parse::<i32>().unwrap_or_default();
                }
                "NetworkType" => {
                    notif.network_type = column.column_data.parse::<i32>().unwrap_or_default();
                }

                _ => continue,
            }
        }
        notif_vec.push(notif);
    }

    let serde_data_result = serde_json::to_value(&notif_vec);
    let serde_data = match serde_data_result {
        Ok(results) => results,
        Err(err) => {
            error!("[srum] Failed to serialize SRUM Notification table: {err:?}");
            return Err(SrumError::Serialize);
        }
    };

    Ok((serde_data, String::from("srum_notification")))
}

#[cfg(test)]
mod tests {
    use super::parse_notification;
    use crate::artifacts::os::windows::{
        ese::parser::grab_ese_tables, srum::tables::index::parse_id_lookup,
    };

    #[test]
    fn test_parse_notification() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";
        let table = vec![
            String::from("SruDbIdMapTable"),
            String::from("{D10CA2FE-6FCF-4F6D-848E-B2E99266FA86}"),
        ];
        let test_data = grab_ese_tables(test_path, &table).unwrap();
        let ids = test_data.get("SruDbIdMapTable").unwrap();
        let id_results = parse_id_lookup(&ids);
        let energy = test_data
            .get("{D10CA2FE-6FCF-4F6D-848E-B2E99266FA86}")
            .unwrap();

        parse_notification(&energy, &id_results).unwrap();
    }
}
