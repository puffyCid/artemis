use crate::{
    artifacts::os::windows::outlook::{
        blocks::descriptors::DescriptorData,
        header::NodeID,
        tables::{context::TableRows, properties::PropertyName, property::PropertyContext},
    },
    utils::{
        compression::decompress::decompress_rtf,
        encoding::{base64_decode_standard, base64_encode_standard},
        nom_helper::{nom_unsigned_four_bytes, Endian},
        strings::{extract_ascii_utf16_string, extract_utf8_string},
    },
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
    props: &mut Vec<PropertyContext>,
    attachments: &Vec<Vec<TableRows>>,
    descriptors: &BTreeMap<u64, DescriptorData>,
) -> MessageDetails {
    let mut message = MessageDetails {
        props: Vec::new(),
        body: String::new(),
        subject: String::new(),
        from: String::new(),
        recipient: String::new(),
        delivered: String::new(),
        attachments: Vec::new(),
        recipients: Vec::new(),
    };

    let mut keep = Vec::new();

    for prop in &mut *props {
        if prop.name.contains(&PropertyName::PidTagHtml) {
            let encoded = prop.value.as_str().unwrap_or_default();

            let decode_result = base64_decode_standard(encoded);
            let decode = match decode_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[outlook] Could not base64 decode HTML message: {err:?}");
                    message.body = encoded.to_string();
                    keep.push(false);
                    continue;
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
            let encoded = prop.value.as_str().unwrap_or_default();
            let data_result = base64_decode_standard(encoded);
            let data = match data_result {
                Ok(result) => result,
                Err(err) => {
                    error!(
                        "[outlook] Failed to decode encoded RTF data: {err:?}. Returning base64 data"
                    );
                    message.body = encoded.to_string();
                    keep.push(false);
                    continue;
                }
            };

            let decom_result = get_rtf_data(&data);
            let decom = match decom_result {
                Ok((_, result)) => result,
                Err(_err) => {
                    error!("[outlook] Failed to parse RTF data. Returning base64 data");
                    message.body = encoded.to_string();
                    keep.push(false);
                    continue;
                }
            };

            message.body = decom;
        } else {
            keep.push(true);
            continue;
        }

        keep.push(false);
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

    let mut iter = keep.iter();
    // Remove all props we already extracted above. We do this so we do not store the body twice
    props.retain(|_| *iter.next().unwrap_or(&false));
    message.props = props.to_vec();

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

fn get_rtf_data(data: &[u8]) -> nom::IResult<&[u8], String> {
    let (input, compression_size) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, uncompressed_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, sig) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, crc) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let compressed_sig = 0x75465a4c;
    if sig != compressed_sig {
        println!("not compressed?: {input:?}");
        // Data is not compressed extract the string new_with_window_bits
        let value = extract_ascii_utf16_string(input);
        println!("{value}");
        panic!("got non compressed RTF?!");

        return Ok((input, value));
    }

    let decom_result = decompress_rtf(input, &uncompressed_size);
    let decom = match decom_result {
        Ok(result) => result,
        Err(err) => {
            panic!("[outlook] Failed to decompress RTF data: {err:?}. Returning base64 data");
            return Ok((input, base64_encode_standard(data)));
        }
    };

    let value = extract_ascii_utf16_string(&decom);

    Ok((input, value))
}
