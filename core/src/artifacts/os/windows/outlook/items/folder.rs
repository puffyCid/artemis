use crate::{
    artifacts::os::windows::outlook::tables::{
        context::{TableContext, TableRows},
        properties::PropertyName,
        property::PropertyContext,
    },
    utils::strings::extract_utf8_string,
};

pub(crate) struct FolderInfo {
    /**Name of the folder */
    pub(crate) name: String,
    /**Timestamp when folder was created */
    pub(crate) created: String,
    /**Timestamp when folder was modified */
    pub(crate) modified: String,
    /**TableRows associated with the Hierarchy (subfolders) */
    hierarchy: Vec<Vec<TableRows>>,
    /**Folder Properties */
    pub(crate) properties: Vec<PropertyContext>,
    /**Subfolders that can be iterated into */
    pub(crate) subfolders: Vec<SubFolder>,
    /**Additional Folder metadata */
    pub(crate) associated_content: Vec<SubFolder>,
    /**Number of subfolders */
    pub(crate) subfolder_count: usize,
    /**Number of messages */
    pub(crate) message_count: usize,
    /**Messages that can be iterated into */
    pub(crate) messages: Vec<MessagePreview>,
}

#[derive(Debug)]
pub(crate) struct SubFolder {
    pub(crate) name: String,
    pub(crate) node: u64,
}

#[derive(Debug)]
pub(crate) struct MessagePreview {
    pub(crate) subject: String,
    pub(crate) delivery: String,
    pub(crate) node: u64,
}

pub(crate) fn folder_details(
    normal: &[PropertyContext],
    hierarchy: &TableContext,
    contents: &TableContext,
    fai: &TableContext,
) -> FolderInfo {
    let mut info = FolderInfo {
        name: String::new(),
        created: String::new(),
        modified: String::new(),
        hierarchy: Vec::new(),
        associated_content: Vec::new(),
        properties: Vec::new(),
        subfolders: Vec::new(),
        subfolder_count: 0,
        message_count: 0,
        messages: Vec::new(),
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
    for rows in &hierarchy.rows {
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
                println!("subject name: {}", sub.name);
                info.subfolders.push(sub);
                break;
            }
        }
    }

    info.subfolder_count = info.subfolders.len();
    info.hierarchy = hierarchy.rows.clone();

    // FAI contains associated folder metadata
    for rows in &fai.rows {
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
                println!("class name: {}", sub.name);
                info.associated_content.push(sub);
                break;
            }
        }
    }

    println!("Contents len: {}", contents.rows.len());

    for rows in &contents.rows {
        let mut mess = MessagePreview {
            subject: String::new(),
            delivery: String::new(),
            node: 0,
        };
        for column in rows {
            if column
                .column
                .property_name
                .contains(&PropertyName::PidTagLtpRowId)
            {
                mess.node = column.value.as_u64().unwrap_or_default();
            } else if column
                .column
                .property_name
                .contains(&PropertyName::PidTagSubjectW)
            {
                let subject = column.value.as_str().unwrap_or_default().to_string();
                let sub_bytes = subject.as_bytes();
                if sub_bytes.starts_with(&[1, 1])
                    || sub_bytes.starts_with(&[1, 4])
                    || sub_bytes.starts_with(&[1, 5])
                    || sub_bytes.starts_with(&[1, 6])
                    || sub_bytes.starts_with(&[1, 7])
                    || sub_bytes.starts_with(&[1, 16])
                    || sub_bytes.starts_with(&[1, 20])
                    || sub_bytes.starts_with(&[1, 26])
                {
                    let clean_subject = extract_utf8_string(&sub_bytes[2..]);
                    mess.subject = clean_subject;
                } else {
                    mess.subject = column.value.as_str().unwrap_or_default().to_string();
                }
            } else if column
                .column
                .property_name
                .contains(&PropertyName::PidTagMessageDeliveryTime)
            {
                mess.delivery = column.value.as_str().unwrap_or_default().to_string();
            }

            if !mess.subject.is_empty() && mess.node != 0 && !mess.delivery.is_empty() {
                println!("message: {mess:?}");
                info.messages.push(mess);
                break;
            }
        }
    }

    info.message_count = info.messages.len();

    info
}

pub(crate) fn search_folder_details(
    search: &[PropertyContext],
    criteria: &[PropertyContext],
    contents: &TableContext,
) -> FolderInfo {
    let mut info = FolderInfo {
        name: String::new(),
        created: String::new(),
        modified: String::new(),
        hierarchy: Vec::new(),
        associated_content: Vec::new(),
        properties: Vec::new(),
        subfolders: Vec::new(),
        subfolder_count: 0,
        message_count: 0,
        messages: Vec::new(),
        // folders: Vec::new(),
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
    info.hierarchy = contents.rows.clone();
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
    use std::io::BufReader;

    #[test]
    fn test_folder_details_root() {
        // We need an OST file for this test
        let reader =
            file_reader("C:\\Users\\bob\\Desktop\\azur3m3m1crosoft@outlook.com.ost").unwrap();
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

        assert_eq!(result.created, "2024-07-29T04:29:52.000Z");
        assert_eq!(result.subfolder_count, 2);
        assert_eq!(result.name, "");
    }
}
