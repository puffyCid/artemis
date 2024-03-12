/**
 * Extensible Storage Engine (`ESE`) is an open source database format used by various Windows applications  
 * Such as: Windows Search (Pre-Win11), Windows Catalog files, BITS, SRUM, Windows Updates, and lots more  
 *
 * Its an extremely complex format, currently we focus on providing the ability to dump table rows which contains the data of interest  
 * Often `ESE` files are locked so we use the NTFS parser to read the files (`raw_read_file`)
 *
 * References:  
 * `https://github.com/libyal/libesedb/blob/main/documentation/Extensible%20Storage%20Engine%20(ESE)%20Database%20File%20(EDB)%20format.asciidoc`
 * `https://github.com/Velocidex/go-ese`
 * `https://techcommunity.microsoft.com/t5/ask-the-directory-services-team/ese-deep-dive-part-1-the-anatomy-of-an-ese-database/ba-p/400496`
 * `https://github.com/microsoft/Extensible-Storage-Engine`
 *
 * Other Parsers:  
 * `https://github.com/Velocidex/velociraptor`
 */
use super::{error::EseError, tables::dump_table};
use common::windows::TableDump;
use log::error;
use std::collections::HashMap;

/**
 * Parse and dump one (1) or more ESE tables from provided bytes
 * Returns a `HashMap` of dumped tables where each table represents the `HashMap` key
 */
pub(crate) fn grab_ese_tables(
    path: &str,
    tables: &[String],
) -> Result<HashMap<String, Vec<Vec<TableDump>>>, EseError> {
    let mut table_data = HashMap::new();

    for table in tables {
        // Dump our table
        let table_result = dump_table(path, table);
        match table_result {
            Ok(result) => {
                // Our hashmap is based on table name for the keys
                if let Some(value) = result.get(table) {
                    table_data.insert(table.clone(), value.clone());
                }
            }
            Err(_err) => {
                error!("[ese] Failed to parse table {table}");
            }
        }
    }

    Ok(table_data)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use crate::artifacts::os::windows::ese::parser::grab_ese_tables;
    use common::windows::ColumnType;
    use std::path::PathBuf;

    #[test]
    fn test_grab_ese_tables() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");

        let results = grab_ese_tables(
            test_location.to_str().unwrap(),
            &vec![
                String::from("MSysObjects"),
                String::from("Jobs"),
                String::from("Files"),
            ],
        )
        .unwrap();

        let catalog = results.get("MSysObjects").unwrap();
        assert_eq!(catalog.len(), 82);

        let job = results.get("Jobs").unwrap();
        assert_eq!(job[0][0].column_name, "Id");
        assert_eq!(job[0][0].column_type, ColumnType::Guid);
        assert_eq!(
            job[0][0].column_data,
            "266504ac-d974-446c-96ad-2be13a5665b0"
        );

        assert_eq!(job[0][1].column_name, "Blob");
        assert_eq!(job[0][1].column_type, ColumnType::LongBinary);
        assert_eq!(job[0][1].column_data.len(), 2740);

        let job = results.get("Files").unwrap();
        assert_eq!(job[0][0].column_name, "Id");
        assert_eq!(job[0][0].column_type, ColumnType::Guid);
        assert_eq!(
            job[0][0].column_data,
            "95d6889c-b2d3-4748-8eb1-9da0650cb892"
        );

        assert_eq!(job[0][1].column_name, "Blob");
        assert_eq!(job[0][1].column_type, ColumnType::LongBinary);
        assert_eq!(job[0][1].column_data.len(), 1432);
    }

    #[test]
    fn test_security_database() {
        let test = "C:\\Windows\\security\\database\\secedit.sdb";
        let results = grab_ese_tables(test, &["SmTblSmp".to_owned()]).unwrap();
        assert!(!results.is_empty());
    }
}
