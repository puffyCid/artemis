use crate::artifacts::os::windows::srum::error::SrumError;
use common::windows::{NotificationInfo, TableDump};
use log::error;
use serde_json::Value;
use std::collections::HashMap;

/// Parse the notification table from SRUM
pub(crate) fn parse_notification(
    column_rows: &[Vec<TableDump>],
    lookups: &HashMap<String, String>,
) -> Result<(Value, String), SrumError> {
    let mut notif_vec: Vec<NotificationInfo> = Vec::new();
    for rows in column_rows {
        let mut notif = NotificationInfo {
            auto_inc_id: 0,
            timestamp: String::new(),
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
                    notif.timestamp.clone_from(&column.column_data);
                }
                "AppId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        notif.app_id.clone_from(value);
                        continue;
                    }
                    notif.app_id.clone_from(&column.column_data);
                }
                "UserId" => {
                    if let Some(value) = lookups.get(&column.column_data) {
                        notif.user_id.clone_from(value);
                        continue;
                    }
                    notif.user_id.clone_from(&column.column_data);
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

                _ => (),
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
#[cfg(target_os = "windows")]
mod tests {
    use super::parse_notification;
    use crate::artifacts::os::windows::srum::{
        resource::get_srum_ese, tables::index::parse_id_lookup,
    };

    #[test]
    fn test_parse_notification() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";

        let indexes = get_srum_ese(test_path, "SruDbIdMapTable").unwrap();
        let lookups = parse_id_lookup(&indexes);
        let srum_data = get_srum_ese(test_path, "{D10CA2FE-6FCF-4F6D-848E-B2E99266FA86}").unwrap();

        parse_notification(&srum_data, &lookups).unwrap();
    }
}
