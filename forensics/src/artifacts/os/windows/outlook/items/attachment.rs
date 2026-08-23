use super::message::{AttachMethod, get_attach_method};
use common::{outlook::PropertyName, windows::PropertyContext};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub(crate) struct Attachment {
    pub(crate) data: String,
    pub(crate) size: u64,
    pub(crate) name: String,
    pub(crate) mime: String,
    pub(crate) extension: String,
    pub(crate) method: AttachMethod,
    pub(crate) props: Vec<PropertyContext>,
}

/// Extract properties associated with Attachments
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
            // Sometimes we have AttachFilenameW property, sometimes we may not. Do not override it
            if !attach.name.is_empty() {
                keep.push(true);
                continue;
            }
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
            attach.method = get_attach_method(method);
        } else if prop.name.contains(&PropertyName::PidTagAttachFilenameW) {
            // Sometimes we have DisplayNameW property, sometimes we may not. Do not override it
            if !attach.name.is_empty() {
                keep.push(true);
                continue;
            }
            attach.name = prop.value.as_str().unwrap_or_default().to_string();
        } else {
            keep.push(true);
            continue;
        }

        keep.push(false);
    }

    let mut iter = keep.iter();
    // Remove all props we already extracted above. We do this so we do not store the attachment twice
    props.retain(|_| *iter.next().unwrap_or(&false));
    attach.props = props.clone();

    attach
}

#[cfg(test)]
mod tests {
    use crate::{
        accessor::access::Accessor,
        artifacts::os::windows::outlook::{
            header::FormatType,
            helper::{OutlookReader, OutlookReaderAction},
            items::message::AttachMethod,
        },
    };
    use std::path::PathBuf;

    #[test]
    fn test_extract_attachment() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/test@outlook.com.ost");

        let reader = Accessor::with_defaults()
            .open_reader(test_location.to_str().unwrap())
            .unwrap();

        let mut outlook_reader = OutlookReader {
            fs: reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unicode64_4k,
            size: 4096,
        };
        outlook_reader.setup().unwrap();

        let attach = outlook_reader.read_attachment(8016, 8010).unwrap();

        assert_eq!(attach.data.len(), 18752);
        assert_eq!(attach.extension, ".png");
        assert_eq!(attach.method, AttachMethod::ByValue);
        assert_eq!(attach.mime, "image/png");
        assert_eq!(attach.name, "wm-tips-mo.png");
        assert_eq!(attach.size, 14247);
        assert_eq!(attach.props.len(), 11);
    }
}
