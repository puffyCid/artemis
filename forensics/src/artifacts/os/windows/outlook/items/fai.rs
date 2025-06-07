use common::{outlook::PropertyName, windows::PropertyContext};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct FolderMeta {
    pub(crate) message_class: String,
    pub(crate) created: String,
    pub(crate) properties: Vec<PropertyContext>,
}

/**
* Extract additional Folder metadata
* Per Microsoft Docs:
* ```
 Folder Associated Information (FAI): A collection of Message objects that are stored in a
 Folder object and are typically hidden from view by email applications. An FAI Message object is
 used to store a variety of settings and auxiliary data, including forms, views, calendar options,
 favorites, and category lists.
* ```
*/
pub(crate) fn extract_fai(info: &[PropertyContext]) -> FolderMeta {
    let mut meta = FolderMeta {
        message_class: String::new(),
        created: String::new(),
        properties: Vec::new(),
    };

    for entry in info {
        if entry.name.contains(&PropertyName::PidTagMessageClassW) {
            meta.message_class = entry.value.as_str().unwrap_or_default().to_string();
        } else if entry.name.contains(&PropertyName::PidTagCreationTime) {
            meta.created = entry.value.as_str().unwrap_or_default().to_string();
        }
    }

    meta.properties = info.to_vec();
    meta
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

        let meta = outlook_reader.folder_metadata(None, 1048616).unwrap();

        assert_eq!(meta.message_class, "IPM.Note");
        assert_eq!(meta.created, "2024-09-10T07:14:33.000Z");
        assert_eq!(meta.properties.len(), 13);
    }
}
