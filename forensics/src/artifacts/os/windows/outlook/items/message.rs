use crate::{
    artifacts::os::windows::outlook::{
        blocks::descriptors::DescriptorData, header::NodeID, tables::context::TableRows,
    },
    utils::{
        compression::decompress::decompress_rtf,
        encoding::{base64_decode_standard, base64_encode_standard},
        nom_helper::{Endian, nom_unsigned_four_bytes},
        strings::{extract_ascii_utf16_string, extract_utf8_string_lossy},
    },
};
use common::{outlook::PropertyName, windows::PropertyContext};
use log::error;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashSet};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct MessageDetails {
    pub(crate) props: Vec<PropertyContext>,
    pub(crate) body: String,
    pub(crate) subject: String,
    pub(crate) from: String,
    pub(crate) recipient: String,
    pub(crate) delivered: String,
    pub(crate) attachments: Vec<AttachmentInfo>,
    pub(crate) recipients: HashSet<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct AttachmentInfo {
    name: String,
    size: u64,
    method: AttachMethod,
    node: u64,
    pub(crate) block_id: u64,
    pub(crate) descriptor_id: u64,
}

#[derive(Debug, PartialEq, Serialize, Deserialize)]

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

/// Get the content of email messages
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
        recipients: HashSet::new(),
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

            message.body = extract_utf8_string_lossy(&decode);
        } else if prop.name.contains(&PropertyName::PidTagMessageDeliveryTime) {
            message.delivered = prop.value.as_str().unwrap_or_default().to_string();
        } else if prop.name.contains(&PropertyName::PidTagSubjectW) {
            let subject = prop.value.as_str().unwrap_or_default().to_string();
            message.subject = clean_subject(&subject);
        } else if prop.name.contains(&PropertyName::PidTagSenderEmailAddressW) {
            message.from = prop.value.as_str().unwrap_or_default().to_string();
        } else if prop.name.contains(&PropertyName::PidTagDisplayToW)
            && message.recipient.is_empty()
        {
            // Defer to PidTagReceivedBySmtpAddress property. But sometimes that property is not present
            message.recipient = prop.value.as_str().unwrap_or_default().to_string();
            // Just in case PidTagReceivedBySmtpAddress overrides this, we will keep the
            // PidTagDisplayToW property
            keep.push(true);
            continue;
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
        } else if prop.name.contains(&PropertyName::PidTagBodyW) {
            // Plaintext email
            message.body = prop.value.as_str().unwrap_or_default().to_string();
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
                info.method = get_attach_method(method);
            }
        }

        message.attachments.push(info);
    }

    for entry in message.attachments.iter_mut() {
        for attach in descriptors.values() {
            if attach.node.node_id != NodeID::Attachment {
                continue;
            }

            if entry.node == attach.node.node as u64 {
                entry.block_id = attach.block_data_id;
                entry.descriptor_id = attach.block_descriptor_id;
            }
        }
    }

    let mut iter = keep.iter();
    // Remove all props we already extracted above. We do this so we do not store the body twice
    props.retain(|_| *iter.next().unwrap_or(&false));
    message.props = props.clone();

    message
}

/// Get info on how an attachment was attached to email
pub(crate) fn get_attach_method(method: u64) -> AttachMethod {
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

/// Extract RTF data from emails. RTF may be compressed
fn get_rtf_data(data: &[u8]) -> nom::IResult<&[u8], String> {
    let (input, _compression_size) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, uncompressed_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, sig) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _crc) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let compressed_sig = 0x75465a4c;
    if sig != compressed_sig {
        // Data is not compressed extract the string
        let value = extract_ascii_utf16_string(input);

        return Ok((input, value));
    }

    let decom_result = decompress_rtf(input, uncompressed_size);
    let decom = match decom_result {
        Ok(result) => result,
        Err(err) => {
            error!("[outlook] Failed to decompress RTF data: {err:?}. Returning base64 data");
            return Ok((input, base64_encode_standard(data)));
        }
    };

    let value = extract_ascii_utf16_string(&decom);

    Ok((input, value))
}

#[derive(Debug)]
pub(crate) struct MessagePreview {
    pub(crate) subject: String,
    pub(crate) delivery: String,
    pub(crate) node: u64,
}

/// Extract some info from the table that points to the messages
pub(crate) fn table_message_preview(rows: &Vec<Vec<TableRows>>) -> Vec<MessagePreview> {
    let mut info = Vec::new();
    for row in rows {
        let mut mess = MessagePreview {
            subject: String::new(),
            delivery: String::new(),
            node: 0,
        };
        for column in row {
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
                mess.subject = clean_subject(&subject);
            } else if column
                .column
                .property_name
                .contains(&PropertyName::PidTagMessageDeliveryTime)
            {
                mess.delivery = column.value.as_str().unwrap_or_default().to_string();
            }

            if !mess.subject.is_empty() && mess.node != 0 && !mess.delivery.is_empty() {
                break;
            }
        }

        info.push(mess);
    }

    info
}

/// Clean subject. Sometimes it has control control
fn clean_subject(sub: &str) -> String {
    let sub_bytes = sub.as_bytes();
    // https://github.com/libyal/libfmapi/blob/main/documentation/MAPI%20definitions.asciidoc#102-subject-control-codes
    if sub_bytes.starts_with(&[1, 1])
        || sub_bytes.starts_with(&[1, 4])
        || sub_bytes.starts_with(&[1, 5])
        || sub_bytes.starts_with(&[1, 6])
        || sub_bytes.starts_with(&[1, 7])
        || sub_bytes.starts_with(&[1, 16])
        || sub_bytes.starts_with(&[1, 20])
        || sub_bytes.starts_with(&[1, 26])
    {
        extract_utf8_string_lossy(&sub_bytes[2..])
    } else {
        sub.to_string()
    }
}

/// Grab other recipients
pub(crate) fn recipients(data: &Vec<Vec<TableRows>>) -> HashSet<String> {
    let mut info = HashSet::new();
    for entry in data {
        for row in entry {
            if !row.value.is_string() || row.value.as_str().is_some_and(|s| !s.contains('@')) {
                continue;
            }

            let address = row.value.as_str().unwrap_or("").to_string();
            info.insert(address.replace('\'', ""));
        }
    }
    info
}

#[cfg(test)]
mod tests {
    use super::get_rtf_data;
    use crate::{
        artifacts::os::windows::outlook::{
            header::FormatType,
            helper::{OutlookReader, OutlookReaderAction},
            items::message::{AttachMethod, clean_subject, get_attach_method},
        },
        filesystem::files::file_reader,
    };
    use std::{io::BufReader, path::PathBuf};

    #[test]
    fn test_message_details() {
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
        let mut folder = outlook_reader.read_folder(None, 8578).unwrap();
        // Read 4th message (in table)
        folder.messages_table.rows = vec![3];
        let messages = outlook_reader
            .read_message(None, &folder.messages_table, None)
            .unwrap();

        assert_eq!(messages[0].body.len(), 11750);
        assert_eq!(
            messages[0].subject,
            "Welcome to your new Outlook.com account"
        );
        assert_eq!(messages[0].from, "no-reply@microsoft.com");
    }

    #[test]
    fn test_get_attach_method() {
        let test = 99;
        assert_eq!(get_attach_method(test), AttachMethod::Unknown);
    }

    #[test]
    fn test_get_rtf_data() {
        let test = [
            219, 0, 0, 0, 71, 1, 0, 0, 76, 90, 70, 117, 83, 82, 121, 25, 97, 0, 10, 102, 98, 105,
            100, 4, 0, 0, 99, 99, 192, 112, 103, 49, 50, 53, 50, 0, 254, 3, 67, 240, 116, 101, 120,
            116, 1, 247, 2, 164, 3, 227, 2, 0, 4, 99, 104, 10, 192, 115, 101, 116, 48, 32, 239, 7,
            109, 2, 131, 0, 80, 17, 77, 50, 10, 128, 6, 180, 2, 128, 150, 125, 10, 128, 8, 200, 59,
            9, 98, 49, 57, 14, 192, 191, 9, 195, 22, 114, 10, 50, 22, 113, 2, 128, 21, 98, 42, 9,
            176, 115, 9, 240, 4, 144, 97, 116, 5, 178, 14, 80, 3, 96, 115, 162, 111, 1, 128, 32,
            69, 120, 17, 193, 110, 24, 48, 93, 6, 82, 118, 4, 144, 23, 182, 2, 16, 114, 0, 192,
            116, 125, 8, 80, 110, 26, 49, 16, 32, 5, 192, 5, 160, 27, 100, 100, 154, 32, 3, 82, 32,
            16, 34, 23, 178, 92, 118, 8, 144, 228, 119, 107, 11, 128, 100, 53, 29, 83, 4, 240, 7,
            64, 13, 23, 112, 48, 10, 113, 23, 242, 98, 107, 109, 107, 6, 115, 1, 144, 0, 32, 32,
            66, 77, 95, 66, 224, 69, 71, 73, 78, 125, 10, 252, 21, 81, 33, 96,
        ];

        let (_, message) = get_rtf_data(&test).unwrap();
        assert_eq!(message.len(), 327);
        assert_eq!(
            message,
            "{\\rtf1\\ansi\\fbidis\\ansicpg1252\\deff0\\deftab720\\fromtext{\\fonttbl{\\f0\\fswiss\\fcharset0 Times New Roman;}{\\f1\\fswiss\\fcharset2\n\rSymbol;}}\n\r{\\colortbl;\\red192\\green192\\blue192;}\n\r{\\*\\generator Microsoft Exchange Server;}\n\r{\\*\\formatConverter converted from text;}\n\r\\viewkind5\\viewscale100\n\r{\\*\\bkmkstart BM_BEGIN}\\pard\\plain\\f0}\n\r"
        );
    }

    #[test]
    fn test_table_message_preview() {
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
        let mut folder = outlook_reader.read_folder(None, 8578).unwrap();
        // Read 6th message (in table)
        folder.messages_table.rows = vec![5];
        let messages = outlook_reader
            .read_message(None, &folder.messages_table, None)
            .unwrap();

        assert_eq!(messages[0].body.len(), 190);
        assert_eq!(messages[0].subject, "Whodunit");
    }

    #[test]
    fn test_clean_subject() {
        let test = "test";
        assert_eq!(clean_subject(&test), "test");
    }
}
