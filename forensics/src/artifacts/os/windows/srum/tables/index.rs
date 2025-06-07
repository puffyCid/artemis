use crate::{
    artifacts::os::windows::securitydescriptor::sid::grab_sid,
    utils::{
        encoding::{base64_decode_standard, base64_encode_standard},
        strings::extract_utf16_string,
    },
};
use common::windows::TableDump;
use log::{error, warn};
use std::collections::HashMap;

/**
 * Before parsing `SRUM` data parse the `SruDbIdMapTable` table which is an Index that contains resolved ID values (ex: SIDs, application names)
 */
pub(crate) fn parse_id_lookup(column_rows: &[Vec<TableDump>]) -> HashMap<String, String> {
    let mut id_lookups = HashMap::new();
    for rows in column_rows {
        let mut col_type = String::new();
        let mut id = String::new();
        let mut blob = Vec::new();
        for column in rows {
            if column.column_name == "IdType" {
                col_type.clone_from(&column.column_data);
            } else if column.column_name == "IdIndex" {
                id.clone_from(&column.column_data);
            } else if column.column_name == "IdBlob" {
                let decode_results = base64_decode_standard(&column.column_data);
                blob = match decode_results {
                    Ok(results) => results,
                    Err(err) => {
                        error!("[srum] Could not base64 decode ID blog: {err:?}");
                        continue;
                    }
                };
            }
        }

        if blob.is_empty() {
            id_lookups.insert(id, String::new());
            continue;
        }
        match col_type.as_str() {
            "3" => {
                let sid_results = grab_sid(&blob);
                let sid = match sid_results {
                    Ok((_, results)) => results,
                    Err(_err) => {
                        warn!("[srum] Could not parse SID ID blob");
                        String::new()
                    }
                };
                id_lookups.insert(id, sid)
            }
            "1" | "2" | "0" => id_lookups.insert(id, extract_utf16_string(&blob)),
            _ => {
                warn!("[srum] Unknown ID Type");
                id_lookups.insert(id, base64_encode_standard(&blob))
            }
        };
    }

    id_lookups
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::parse_id_lookup;
    use crate::artifacts::os::windows::srum::resource::get_srum_ese;

    #[test]
    fn test_parse_id_lookup() {
        let test_path = "C:\\Windows\\System32\\sru\\SRUDB.dat";

        let indexes = get_srum_ese(test_path, "SruDbIdMapTable").unwrap();

        let results = parse_id_lookup(&indexes);
        assert!(results.len() > 20);
    }
}
