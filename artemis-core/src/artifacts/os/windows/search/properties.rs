use crate::artifacts::os::windows::ese::parser::TableDump;
use std::collections::HashMap;

/**
 * Windows `Search` entries have a ton of properties (almost 600).
 * Each of these properties have their own column. We gather all non-empty properties into a hashmap for later lookups
 */
pub(crate) fn parse_prop_id_lookup(
    column_rows: &[Vec<TableDump>],
) -> HashMap<String, HashMap<String, String>> {
    let mut id_lookups = HashMap::new();

    for rows in column_rows {
        let mut id = String::new();

        let mut props = HashMap::new();
        for column in rows {
            if column.column_name == "WorkID" {
                id = column.column_data.clone();
                continue;
            }

            if column.column_data.is_empty() {
                continue;
            }

            props.insert(column.column_name.clone(), column.column_data.clone());
        }

        id_lookups.insert(id, props);
    }

    id_lookups
}

#[cfg(test)]
mod tests {
    use super::parse_prop_id_lookup;
    use crate::{artifacts::os::windows::ese::parser::grab_ese_tables, filesystem::files::is_file};

    #[test]
    #[ignore = "Can take a long time"]
    fn test_parse_prop_id_lookup() {
        let test_path =
            "C:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb";
        // Some versions of Windows 11 do not use ESE for Windows Search
        if !is_file(test_path) {
            return;
        }
        let table = vec![String::from("SystemIndex_PropertyStore")];
        let test_data = grab_ese_tables(test_path, &table).unwrap();
        let ids = test_data.get("SystemIndex_PropertyStore").unwrap();
        let results = parse_prop_id_lookup(&ids);
        assert!(results.len() > 20);
    }
}
