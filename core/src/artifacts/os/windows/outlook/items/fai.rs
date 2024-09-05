use crate::artifacts::os::windows::outlook::tables::{
    properties::PropertyName, property::PropertyContext,
};

#[derive(Debug)]
pub(crate) struct FolderMeta {
    message_class: String,
    created: String,
    properties: Vec<PropertyContext>,
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
