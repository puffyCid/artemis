use super::ids::property_name_ids;
use crate::{
    artifacts::os::windows::outlook::{
        error::OutlookError,
        tables::{properties::PropertyName, property::PropertyContext},
    },
    utils::{
        encoding::base64_decode_standard,
        nom_helper::{nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian},
        strings::extract_ascii_utf16_string,
        uuid::format_guid_le_bytes,
    },
};
use log::{error, warn};
use nom::bytes::complete::take;
use serde_json::Value;
use std::collections::HashMap;

pub(crate) fn extract_name_id_map(
    context: &[PropertyContext],
) -> Result<HashMap<u16, NameEntry>, OutlookError> {
    let name_props = vec![
        PropertyName::StreamGuid,
        PropertyName::StreamEntry,
        PropertyName::StreamString,
    ];
    let mut guids = Vec::new();
    let mut strings = Vec::new();
    let mut entries = Vec::new();

    // Get the NameIdMap data from the PropertyContext.
    // There are three (3) properties we need: StreamEntries, StreamString, StreamEntry, StreamGuid
    // (Technically) StreamEntries is not required
    for entry in context {
        let bytes_result = if entry.name.iter().any(|item| name_props.contains(item)) {
            base64_decode_standard(entry.value.as_str().unwrap_or_default())
        } else {
            continue;
        };

        let bytes = match bytes_result {
            Ok(result) => result,
            Err(err) => {
                error!("[outlook] Could not base64 name-id-map property data: {err:?}");
                return Err(OutlookError::Base64Property);
            }
        };

        if entry.name.contains(&PropertyName::StreamGuid) {
            let guid_result = name_guids(&bytes);
            guids = match guid_result {
                Ok((_, result)) => result,
                Err(_err) => {
                    error!("[outlook] Failed to parse name-id-map GUIDs data");
                    return Err(OutlookError::NameIdMap);
                }
            };
        } else if entry.name.contains(&PropertyName::StreamString) {
            strings = bytes;
        } else if entry.name.contains(&PropertyName::StreamEntry) {
            let result = name_entries(&bytes);
            entries = match result {
                Ok((_, result)) => result,
                Err(_err) => {
                    error!("[outlook] Could not extract entries from NameMapID");
                    return Err(OutlookError::NameIdMap);
                }
            };
        }
    }

    let mut name_map = HashMap::new();
    // Now go through the entries and get the associated data. Either string or GUID
    for entry in entries.iter_mut() {
        if entry.name_type == NameType::Guid {
            let name = property_name_ids(&entry.reference);
            entry.value = serde_json::to_value(&name).unwrap_or_default();
        } else {
            let string_result = name_string(&strings, &entry.reference);
            let name = match string_result {
                Ok((_, result)) => result,
                Err(_err) => {
                    warn!("[outlook] Could not extract NameIdMap string for: {entry:?}");
                    String::from("Failed to extract string for NameIdMap")
                }
            };

            entry.value = serde_json::to_value(&name).unwrap_or_default();
        }

        if entry.entry_type == 0 || entry.entry_type == 1 {
            entry.guid = String::from("No Class");
        } else if entry.entry_type == 2 || entry.entry_type == 3 {
            entry.guid = String::from("00020328-0000-0000-c000-000000000046");
        } else if entry.entry_type == 4 || entry.entry_type == 5 {
            entry.guid = String::from("00020329-0000-0000-c000-000000000046");
        } else {
            // GUID is found in our GUID array
            let guid_index = (entry.entry_type / 2) - 3;
            if let Some(guid) = guids.get(guid_index as usize) {
                entry.guid.clone_from(guid);
            }
        }

        name_map.insert(entry.entry_number, entry.clone());
    }

    Ok(name_map)
}

fn name_guids(data: &[u8]) -> nom::IResult<&[u8], Vec<String>> {
    let mut input = data;
    let min_guid_size = 16;

    let mut guids = Vec::new();
    while input.len() >= min_guid_size {
        let (remaining, guid_bytes) = take(min_guid_size)(input)?;
        let guid = format_guid_le_bytes(guid_bytes);
        guids.push(guid);

        input = remaining;
    }

    Ok((input, guids))
}

fn name_string<'a>(data: &'a [u8], offset: &u32) -> nom::IResult<&'a [u8], String> {
    let (string_start, _) = take(*offset)(data)?;
    let (input, string_size) = nom_unsigned_four_bytes(string_start, Endian::Le)?;

    let (input, string_data) = take(string_size)(input)?;

    // Strings can either be ASCII or UTF16
    let string = extract_ascii_utf16_string(string_data);

    Ok((input, string))
}

#[derive(Debug, Clone)]
pub(crate) struct NameEntry {
    reference: u32,
    entry_type: u16,
    entry_number: u16,
    value: Value,
    index: u16,
    guid: String,
    name_type: NameType,
}

#[derive(Debug, PartialEq, Clone)]
enum NameType {
    String,
    Guid,
}

/// Extract Name to ID map entries
fn name_entries(data: &[u8]) -> nom::IResult<&[u8], Vec<NameEntry>> {
    let mut input = data;
    let entry_size = 8;

    let mut entries = Vec::new();
    while input.len() >= entry_size {
        let (remaining, reference) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (remaining, entry_type) = nom_unsigned_two_bytes(remaining, Endian::Le)?;
        let (remaining, entry_number) = nom_unsigned_two_bytes(remaining, Endian::Le)?;
        input = remaining;

        let name_type = 1;
        let is_guid = 0;

        let index = 0xffff & (entry_type >> name_type);

        let name_type = if (entry_type & name_type) != is_guid {
            NameType::String
        } else {
            NameType::Guid
        };

        let entry = NameEntry {
            reference,
            entry_type,
            entry_number: entry_number + 0x8000,
            value: Value::Null,
            guid: String::new(),
            index,
            name_type,
        };

        entries.push(entry);
    }

    Ok((input, entries))
}

#[cfg(test)]
mod tests {
    use super::{name_entries, name_guids, name_string};
    use crate::{
        artifacts::os::windows::outlook::{
            header::FormatType,
            helper::{OutlookReader, OutlookReaderAction},
            items::name_map::extract_name_id_map,
            pages::btree::{BlockType, LeafBlockData},
            tables::property::OutlookPropertyContext,
        },
        filesystem::files::{file_reader, read_file},
    };
    use std::{io::BufReader, path::PathBuf};

    #[test]
    fn test_extract_name_id_map() {
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
        let mut leaf_block = LeafBlockData {
            block_type: BlockType::Internal,
            index_id: 0,
            index: 0,
            block_offset: 0,
            size: 0,
            total_size: 0,
            reference_count: 0,
        };

        let mut leaf_descriptor = LeafBlockData {
            block_type: BlockType::Internal,
            index_id: 0,
            index: 0,
            block_offset: 0,
            size: 0,
            total_size: 0,
            reference_count: 0,
        };

        let node_data = outlook_reader.node_btree[0].btree.get(&97).unwrap();
        for blocks in outlook_reader.block_btree.iter() {
            if let Some(block_data) = blocks.get(&node_data.block_offset_data_id) {
                leaf_block = block_data.clone();
            }
            if let Some(block_data) = blocks.get(&node_data.block_offset_descriptor_id) {
                leaf_descriptor = block_data.clone();
            }

            if leaf_descriptor.size != 0 && leaf_block.size != 0 {
                break;
            }
        }

        let block_value = outlook_reader
            .get_block_data(None, &leaf_block, Some(&leaf_descriptor))
            .unwrap();
        let props = outlook_reader
            .parse_property_context(None, &block_value.data, &block_value.descriptors)
            .unwrap();
        assert_eq!(props[1].value.as_str().unwrap().len(), 940);

        let results = extract_name_id_map(&props).unwrap();
        assert_eq!(results.len(), 1276);
    }

    #[test]
    fn test_name_guids() {
        let test = [
            8, 32, 6, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 70, 2, 32, 6, 0, 0, 0, 0, 0, 192, 0, 0,
            0, 0, 0, 0, 70, 134, 3, 2, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 70, 20, 32, 6, 0, 0,
            0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 70, 3, 32, 6, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 70,
            96, 242, 182, 88, 81, 2, 147, 66, 151, 55, 46, 242, 49, 135, 248, 157, 144, 218, 216,
            110, 11, 69, 27, 16, 152, 218, 0, 170, 0, 63, 19, 5, 4, 32, 6, 0, 0, 0, 0, 0, 192, 0,
            0, 0, 0, 0, 0, 70, 10, 32, 6, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 70, 64, 32, 6, 0,
            0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 70, 19, 143, 242, 65, 244, 131, 20, 65, 165, 132,
            238, 219, 90, 107, 11, 255, 24, 185, 80, 229, 89, 152, 185, 71, 128, 149, 151, 228,
            231, 47, 25, 38, 31, 164, 235, 51, 168, 122, 46, 66, 190, 123, 121, 225, 169, 142, 84,
            179, 8, 150, 35, 35, 93, 104, 50, 71, 156, 85, 76, 149, 203, 78, 142, 51, 127, 127, 53,
            150, 225, 89, 208, 71, 153, 167, 70, 81, 92, 24, 59, 84, 11, 32, 6, 0, 0, 0, 0, 0, 192,
            0, 0, 0, 0, 0, 0, 70, 65, 32, 6, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 70, 14, 32, 6,
            0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 70, 19, 32, 6, 0, 0, 0, 0, 0, 192, 0, 0, 0, 0, 0,
            0, 70, 128, 118, 58, 78, 122, 183, 208, 17, 157, 165, 0, 192, 79, 214, 86, 133, 16,
            239, 250, 148, 71, 249, 208, 17, 128, 14, 0, 160, 201, 13, 200, 219, 85, 171, 243, 41,
            77, 85, 208, 17, 169, 124, 0, 160, 201, 17, 245, 10, 82, 171, 243, 41, 77, 85, 208, 17,
            169, 124, 0, 160, 201, 17, 245, 10, 83, 171, 243, 41, 77, 85, 208, 17, 169, 124, 0,
            160, 201, 17, 245, 10, 96, 171, 243, 41, 77, 85, 208, 17, 169, 124, 0, 160, 201, 17,
            245, 10, 86, 171, 243, 41, 77, 85, 208, 17, 169, 124, 0, 160, 201, 17, 245, 10, 97,
            171, 243, 41, 77, 85, 208, 17, 169, 124, 0, 160, 201, 17, 245, 10, 48, 241, 37, 183,
            239, 71, 26, 16, 165, 241, 2, 96, 140, 158, 235, 172, 144, 28, 105, 73, 23, 126, 26,
            16, 169, 28, 8, 0, 43, 46, 205, 169, 192, 54, 12, 86, 58, 80, 207, 17, 186, 161, 0, 0,
            76, 117, 42, 154, 150, 245, 43, 200, 49, 184, 208, 17, 183, 51, 0, 170, 0, 161, 235,
            210, 80, 227, 99, 11, 204, 156, 208, 17, 188, 219, 0, 128, 95, 204, 206, 4, 67, 227,
            99, 11, 204, 156, 208, 17, 188, 219, 0, 128, 95, 204, 206, 4, 240, 211, 181, 209, 179,
            192, 207, 17, 154, 146, 0, 160, 201, 8, 219, 241, 16, 122, 235, 112, 217, 85, 207, 17,
            183, 91, 0, 170, 0, 81, 254, 32, 160, 0, 244, 49, 7, 253, 207, 17, 185, 189, 0, 170, 0,
            61, 177, 142, 224, 133, 159, 242, 249, 79, 104, 16, 171, 145, 8, 0, 43, 39, 179, 217,
            2, 213, 205, 213, 156, 46, 27, 16, 147, 151, 8, 0, 43, 44, 249, 174, 7, 14, 0, 17, 27,
            181, 214, 64, 175, 33, 202, 168, 94, 218, 177, 208, 249, 30, 138, 169, 64, 255, 11, 71,
            160, 215, 77, 125, 206, 106, 100, 98, 89, 226, 25, 167, 154, 42, 184, 79, 186, 179, 58,
            159, 2, 151, 14, 75, 18, 224, 141, 246, 239, 236, 142, 78, 190, 181, 209, 222, 91, 8,
            226, 174, 116, 119, 65, 26, 121, 71, 193, 71, 152, 81, 228, 32, 87, 73, 95, 202, 107,
            197, 63, 64, 48, 205, 197, 71, 134, 248, 237, 233, 227, 90, 2, 43,
        ];

        let (_, guids) = name_guids(&test).unwrap();
        assert_eq!(guids.len(), 44);
    }

    #[test]
    fn test_name_entries() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/stream_entry.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let (_, entries) = name_entries(&data).unwrap();
        assert_eq!(entries.len(), 1307);
    }

    #[test]
    fn test_name_string() {
        let test = [
            26, 0, 0, 0, 99, 0, 111, 0, 110, 0, 116, 0, 101, 0, 110, 0, 116, 0, 45, 0, 99, 0, 108,
            0, 97, 0, 115, 0, 115, 0,
        ];

        let (_, string) = name_string(&test, &0).unwrap();
        assert_eq!(string, "content-class");
    }
}
