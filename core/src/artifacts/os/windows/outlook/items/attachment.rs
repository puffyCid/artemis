use super::message::{get_attach_method, AttachMethod};
use crate::artifacts::os::windows::outlook::tables::{
    properties::PropertyName, property::PropertyContext,
};

#[derive(Debug)]
pub(crate) struct Attachment {
    data: String,
    size: u64,
    name: String,
    mime: String,
    extension: String,
    method: AttachMethod,
    props: Vec<PropertyContext>,
}

pub(crate) fn extract_attachment(props: &mut Vec<PropertyContext>) -> Attachment {
    let mut attach = Attachment {
        data: String::new(),
        size: 0,
        name: String::new(),
        mime: String::new(),
        extension: String::new(),
        method: AttachMethod::Unknown,
        props: Vec::new(),
    };

    let mut keep = Vec::new();
    for prop in &mut *props {
        if prop.name.contains(&PropertyName::PidTagDisplayNameW) {
            attach.name = prop.value.as_str().unwrap_or_default().to_string();
        } else if prop.name.contains(&PropertyName::PidTagAttachSize) {
            attach.size = prop.value.as_u64().unwrap_or_default();
        } else if prop.name.contains(&PropertyName::PidTagAttachExtensionW) {
            attach.extension = prop.value.as_str().unwrap_or_default().to_string();
        } else if prop.name.contains(&PropertyName::PidTagAttachMimeTagW) {
            attach.mime = prop.value.as_str().unwrap_or_default().to_string();
        } else if prop.name.contains(&PropertyName::PidTagAttachDataBinary) {
            attach.data = prop.value.as_str().unwrap_or_default().to_string();
        } else if prop.name.contains(&PropertyName::PidTagAttachMethod) {
            let method = prop.value.as_u64().unwrap_or_default();
            attach.method = get_attach_method(&method);
        } else {
            keep.push(true);
            continue;
        }

        keep.push(false);
    }

    let mut iter = keep.iter();
    // Remove all props we already extracted above. We do this so we do not store the attachment twice
    props.retain(|_| *iter.next().unwrap_or(&false));
    attach.props = props.to_vec();

    attach
}
