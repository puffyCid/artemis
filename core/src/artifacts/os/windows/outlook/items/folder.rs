use crate::artifacts::os::windows::outlook::{
    header::NodeID,
    tables::{
        context::{TableInfo, TableRows},
        header::HeapNode,
    },
};
use common::{outlook::PropertyName, windows::PropertyContext};
use std::collections::BTreeMap;

#[derive(Debug)]
pub(crate) struct FolderInfo {
    /**Name of the folder */
    pub(crate) name: String,
    /**Timestamp when folder was created */
    pub(crate) created: String,
    /**Timestamp when folder was modified */
    pub(crate) modified: String,
    /**Folder Properties */
    pub(crate) properties: Vec<PropertyContext>,
    /**Subfolders that can be iterated into */
    pub(crate) subfolders: Vec<SubFolder>,
    /**Additional Folder metadata */
    pub(crate) associated_content: Vec<SubFolder>,
    /**Number of subfolders */
    pub(crate) subfolder_count: usize,
    /**Number of messages */
    pub(crate) message_count: u64,
    /**Messages that can be iterated into */
    pub(crate) messages_table: TableInfo,
}

#[derive(Debug)]
pub(crate) struct SubFolder {
    pub(crate) name: String,
    pub(crate) node: u64,
}

/// Get details on Outlook folders
pub(crate) fn folder_details(
    normal: &[PropertyContext],
    hierarchy: &Vec<Vec<TableRows>>,
    contents: &TableInfo,
    fai: &Vec<Vec<TableRows>>,
) -> FolderInfo {
    let mut info = FolderInfo {
        name: String::new(),
        created: String::new(),
        modified: String::new(),
        associated_content: Vec::new(),
        properties: Vec::new(),
        subfolders: Vec::new(),
        subfolder_count: 0,
        message_count: 0,
        messages_table: contents.clone(),
    };

    for props in normal {
        if props.name.contains(&PropertyName::PidTagDisplayNameW) {
            info.name = props.value.as_str().unwrap_or_default().to_string();
        } else if props.name.contains(&PropertyName::PidTagCreationTime) {
            info.created = props.value.as_str().unwrap_or_default().to_string();
        } else if props
            .name
            .contains(&PropertyName::PidTagLastModificationTime)
        {
            info.modified = props.value.as_str().unwrap_or_default().to_string();
        }
    }

    info.properties = normal.to_vec();

    // Now get any subfolders!
    for rows in hierarchy {
        let mut sub = SubFolder {
            name: String::new(),
            node: 0,
        };
        for columns in rows {
            if columns
                .column
                .property_name
                .contains(&PropertyName::PidTagDisplayNameW)
            {
                sub.name = columns.value.as_str().unwrap_or_default().to_string();
            } else if columns
                .column
                .property_name
                .contains(&PropertyName::PidTagLtpRowId)
            {
                sub.node = columns.value.as_u64().unwrap_or_default();
            }

            if !sub.name.is_empty() && sub.node != 0 {
                info.subfolders.push(sub);
                break;
            }
        }
    }

    info.subfolder_count = info.subfolders.len();

    // FAI contains associated folder metadata
    for rows in fai {
        let mut sub = SubFolder {
            name: String::new(),
            node: 0,
        };
        for column in rows {
            if column
                .column
                .property_name
                .contains(&PropertyName::PidTagLtpRowId)
            {
                sub.node = column.value.as_u64().unwrap_or_default();
            } else if column
                .column
                .property_name
                .contains(&PropertyName::PidTagMessageClassW)
            {
                sub.name = column.value.as_str().unwrap_or_default().to_string();
            }

            if !sub.name.is_empty() && sub.node != 0 {
                info.associated_content.push(sub);
                break;
            }
        }
    }

    info.message_count = info.messages_table.total_rows;

    info
}

/// Get details Outlook search folders. These are special folders in Outlook
pub(crate) fn search_folder_details(
    search: &[PropertyContext],
    criteria: &[PropertyContext],
) -> FolderInfo {
    let mut info = FolderInfo {
        name: String::new(),
        created: String::new(),
        modified: String::new(),
        associated_content: Vec::new(),
        properties: Vec::new(),
        subfolders: Vec::new(),
        subfolder_count: 0,
        message_count: 0,
        messages_table: TableInfo {
            block_data: Vec::new(),
            block_descriptors: BTreeMap::new(),
            rows: Vec::new(),
            columns: Vec::new(),
            include_cols: Vec::new(),
            row_size: 0,
            map_offset: 0,
            node: HeapNode {
                node: NodeID::Unknown,
                index: 0,
                block_index: 0,
            },
            total_rows: 0,
            has_branch: None,
        },
    };

    for props in search {
        if props.name.contains(&PropertyName::PidTagDisplayNameW) {
            info.name = props.value.as_str().unwrap_or_default().to_string();
            // info.folders.push(info.name);
        } else if props.name.contains(&PropertyName::PidTagCreationTime) {
            info.created = props.value.as_str().unwrap_or_default().to_string();
        } else if props
            .name
            .contains(&PropertyName::PidTagLastModificationTime)
        {
            info.modified = props.value.as_str().unwrap_or_default().to_string();
        }
    }

    info.properties = search.to_vec();
    info.properties.append(&mut criteria.to_vec());
    info
}
#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::outlook::{
            header::FormatType,
            helper::{OutlookReader, OutlookReaderAction},
        },
        filesystem::files::file_reader,
    };
    use std::{io::BufReader, path::PathBuf};

    #[test]
    fn test_folder_details_root() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/test@outlook.com.ost");

        let reader = file_reader(test_location.to_str().unwrap()).unwrap();
        let buf_reader = BufReader::new(reader);

        let mut outlook_reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unicode64_4k,
            size: 4096,
        };
        outlook_reader.setup(None).unwrap();

        let result = outlook_reader.root_folder(None).unwrap();

        assert_eq!(result.created, "2024-09-10T07:14:31.000Z");
        assert_eq!(result.subfolder_count, 2);
        assert_eq!(result.name, "");
    }

    #[test]
    fn test_search_folder_details() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/test@outlook.com.ost");

        let reader = file_reader(test_location.to_str().unwrap()).unwrap();
        let buf_reader = BufReader::new(reader);

        let mut outlook_reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unicode64_4k,
            size: 4096,
        };
        outlook_reader.setup(None).unwrap();

        let result = outlook_reader.search_folder(None, 524355).unwrap();

        assert_eq!(result.name, "Reminders");
        assert_eq!(result.created, "2024-09-10T07:15:07.000Z");
        assert_eq!(result.modified, "2024-09-10T07:15:07.000Z");

        assert_eq!(result.properties.len(), 34);
        assert_eq!(result.messages_table.block_data.len(), 0);
    }
}
