use common::windows::TableDump;
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
                id.clone_from(&column.column_data);
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
    use crate::{
        artifacts::os::windows::{
            ese::{helper::get_page_data, tables::table_info},
            search::ese::{get_document_ids, get_properties, search_catalog, search_pages},
        },
        filesystem::files::is_file,
    };

    #[test]
    fn test_parse_prop_id_lookup() {
        let test_path =
            "C:\\ProgramData\\Microsoft\\Search\\Data\\Applications\\Windows\\Windows.edb";
        // Some versions of Windows 11 do not use ESE for Windows Search
        if !is_file(test_path) {
            return;
        }
        let catalog = search_catalog(test_path).unwrap();

        let mut gather_table = table_info(&catalog, "SystemIndex_Gthr");
        let gather_pages = search_pages(gather_table.table_page as u32, test_path).unwrap();

        let mut property_table = table_info(&catalog, "SystemIndex_PropertyStore");
        let property_pages = search_pages(property_table.table_page as u32, test_path).unwrap();

        let page_limit = 1;
        let mut gather_chunk = Vec::new();
        let last_page = 0;
        for gather_page in gather_pages {
            if gather_page == last_page {
                continue;
            }

            gather_chunk.push(gather_page);
            if gather_chunk.len() != page_limit {
                continue;
            }

            let gather_rows = get_page_data(
                test_path,
                &gather_chunk,
                &mut gather_table,
                "SystemIndex_Gthr",
            )
            .unwrap();

            let mut doc_ids = get_document_ids(
                &gather_rows
                    .get("SystemIndex_Gthr")
                    .unwrap_or(&Vec::new())
                    .to_vec(),
            );
            let props = get_properties(
                test_path,
                &property_pages,
                &mut property_table,
                &mut doc_ids,
            );

            let _ = parse_prop_id_lookup(props.get("SystemIndex_PropertyStore").unwrap());
            break;
        }
    }
}
