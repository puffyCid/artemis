use crate::{
    artifacts::os::windows::outlook::{
        blocks::descriptors::DescriptorData,
        header::NodeID,
        tables::{context::TableRows, properties::PropertyName, property::PropertyContext},
    },
    utils::{encoding::base64_decode_standard, strings::extract_utf8_string},
};
use log::error;
use std::collections::BTreeMap;

#[derive(Debug)]
pub(crate) struct MessageDetails {
    pub(crate) props: Vec<PropertyContext>,
    pub(crate) body: String,
    subject: String,
    from: String,
    recipient: String,
    pub(crate) delivered: String,
    pub(crate) attachments: Vec<AttachmentInfo>,
    pub(crate) recipients: Vec<Vec<TableRows>>,
}

#[derive(Debug)]
pub(crate) struct AttachmentInfo {
    name: String,
    size: u64,
    method: AttachMethod,
    node: u64,
    pub(crate) block_id: u64,
    pub(crate) descriptor_id: u64,
}

#[derive(Debug)]

pub(crate) enum AttachMethod {
    None,
    ByValue,
    ByReference,
    ByReferenceResolve,
    ReferenceOnly,
    Embedded,
    Ole,
    Unknown,
}

pub(crate) fn message_details(
    props: &[PropertyContext],
    attachments: &Vec<Vec<TableRows>>,
    descriptors: &BTreeMap<u64, DescriptorData>,
) -> MessageDetails {
    let mut message = MessageDetails {
        props: props.to_vec(),
        body: String::new(),
        subject: String::new(),
        from: String::new(),
        recipient: String::new(),
        delivered: String::new(),
        attachments: Vec::new(),
        recipients: Vec::new(),
    };

    for prop in props {
        if prop.name.contains(&PropertyName::PidTagHtml) {
            let decode_result = base64_decode_standard(prop.value.as_str().unwrap_or_default());
            let decode = match decode_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[outlook] Could not base64 HTML message: {err:?}");
                    Vec::new()
                }
            };

            message.body = extract_utf8_string(&decode);
        } else if prop.name.contains(&PropertyName::PidTagMessageDeliveryTime) {
            message.delivered = prop.value.as_str().unwrap_or_default().to_string();
        } else if prop.name.contains(&PropertyName::PidTagSubjectW) {
            message.subject = prop.value.as_str().unwrap_or_default().to_string();
        } else if prop.name.contains(&PropertyName::PidTagSenderEmailAddressW) {
            message.from = prop.value.as_str().unwrap_or_default().to_string();
        } else if prop
            .name
            .contains(&PropertyName::PidTagReceivedBySmtpAddress)
        {
            message.recipient = prop.value.as_str().unwrap_or_default().to_string();
        } else if prop.name.contains(&PropertyName::PidTagRtfCompressed) {
            panic!("ugh: {:?}", prop.value);
            message.delivered = prop.value.as_str().unwrap_or_default().to_string();
        }
    }

    for attach in attachments {
        let mut info = AttachmentInfo {
            name: String::new(),
            size: 0,
            method: AttachMethod::Unknown,
            node: 0,
            block_id: 0,
            descriptor_id: 0,
        };
        for column in attach {
            if column
                .column
                .property_name
                .contains(&PropertyName::PidTagAttachFilenameW)
            {
                info.name = column.value.as_str().unwrap_or_default().to_string();
            } else if column
                .column
                .property_name
                .contains(&PropertyName::PidTagAttachSize)
            {
                info.size = column.value.as_u64().unwrap_or_default();
            } else if column
                .column
                .property_name
                .contains(&PropertyName::PidTagLtpRowId)
            {
                info.node = column.value.as_u64().unwrap_or_default();
            } else if column
                .column
                .property_name
                .contains(&PropertyName::PidTagAttachMethod)
            {
                let method = column.value.as_u64().unwrap_or_default();
                info.method = get_attach_method(&method);
            }
        }

        message.attachments.push(info);
    }

    for entry in message.attachments.iter_mut() {
        for attach in descriptors.values() {
            if attach.node.node_id != NodeID::Attachment {
                continue;
            }

            entry.block_id = attach.block_data_id;
            entry.descriptor_id = attach.block_descriptor_id;
        }
    }

    message
}

pub(crate) fn get_attach_method(method: &u64) -> AttachMethod {
    match method {
        0 => AttachMethod::None,
        1 => AttachMethod::ByValue,
        2 => AttachMethod::ByReference,
        3 => AttachMethod::ByReferenceResolve,
        4 => AttachMethod::ReferenceOnly,
        5 => AttachMethod::Embedded,
        6 => AttachMethod::Ole,
        _ => AttachMethod::Unknown,
    }
}
