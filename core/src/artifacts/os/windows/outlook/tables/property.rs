use super::{
    context::{get_property_type, PropertyType},
    properties::{property_id_to_name, PropertyName},
};
use crate::{
    artifacts::os::windows::outlook::{
        blocks::descriptors::DescriptorData,
        error::OutlookError,
        helper::{OutlookReader, OutlookReaderAction},
        pages::btree::{BlockType, LeafBlockData, NodeLevel},
        tables::{header::table_header, heap_btree::parse_btree_heap},
    },
    utils::{
        encoding::base64_encode_standard,
        nom_helper::{
            nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
            nom_unsigned_two_bytes, Endian,
        },
        strings::extract_ascii_utf16_string,
        time::{filetime_to_unixepoch, ole_automationtime_to_unixepoch, unixepoch_to_iso},
        uuid::format_guid_le_bytes,
    },
};
use log::{error, warn};
use nom::{
    bytes::complete::take,
    error::ErrorKind,
    number::complete::{le_f32, le_f64},
};
use ntfs::NtfsFile;
use serde_json::Value;
use std::collections::BTreeMap;

/// Property Context Table (also called 0xbc table)
#[derive(Debug, Clone)]
pub(crate) struct PropertyContext {
    pub(crate) name: Vec<PropertyName>,
    pub(crate) property_type: PropertyType,
    pub(crate) prop_id: u16,
    pub(crate) property_number: u16,
    pub(crate) reference: u32,
    pub(crate) value: Value,
}

pub(crate) trait OutlookPropertyContext<T: std::io::Seek + std::io::Read> {
    fn parse_property_context(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        block_data: &[Vec<u8>],
        block_descriptors: &BTreeMap<u64, DescriptorData>,
    ) -> Result<Vec<PropertyContext>, OutlookError>;
    fn get_property_context<'a>(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        header_block: &'a [u8],
        all_blocks: &[Vec<u8>],
        block_descriptors: &BTreeMap<u64, DescriptorData>,
    ) -> nom::IResult<&'a [u8], Vec<PropertyContext>>;

    fn get_large_data(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        block_descriptors: &BTreeMap<u64, DescriptorData>,
        reference: &u32,
    ) -> Result<Vec<Vec<u8>>, OutlookError>;
}

impl<T: std::io::Seek + std::io::Read> OutlookPropertyContext<T> for OutlookReader<T> {
    /// Parse property data
    fn parse_property_context(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        block_data: &[Vec<u8>],
        block_descriptors: &BTreeMap<u64, DescriptorData>,
    ) -> Result<Vec<PropertyContext>, OutlookError> {
        let first_block = block_data.first();
        let block = match first_block {
            Some(result) => result,
            None => return Err(OutlookError::NoBlocks),
        };

        let props_result =
            self.get_property_context(ntfs_file, block, block_data, block_descriptors);
        let props = match props_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[outlook] Could not parse property context");
                return Err(OutlookError::PropertyContext);
            }
        };

        Ok(props)
    }

    /// Parse the Property Context data
    fn get_property_context<'a>(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        header_block: &'a [u8],
        all_blocks: &[Vec<u8>],
        block_descriptors: &BTreeMap<u64, DescriptorData>,
    ) -> nom::IResult<&'a [u8], Vec<PropertyContext>> {
        let (input, header) = table_header(header_block)?;
        let (_, heap_btree) = parse_btree_heap(input)?;

        // Have not seen Branch nodes for properties but maybe they exist?
        if heap_btree.level == NodeLevel::BranchNode {
            warn!("[outlook] Got Branch node for property data. Parse will likely fail");
        }

        let mut prop_data_size = 0;
        // Often property data starts at offset 20 (0x14). But this is not 100% guaranteed
        let mut prop_start = 20;

        // Heap BTree tells us where the property data starts at in the allocation_table
        if let Some(start) = header
            .page_map
            .allocation_table
            .get(heap_btree.node.index as usize - 1)
        {
            if let Some(end) = header
                .page_map
                .allocation_table
                .get(heap_btree.node.index as usize)
            {
                prop_data_size = end - start;
            }
            prop_start = *start;
        }

        let (prop_data_start, _) = take(prop_start)(header_block)?;

        let (_, mut props) = take(prop_data_size)(prop_data_start)?;
        let prop_entry_size = 8;

        if props.len() % prop_entry_size != 0 {
            error!("[outlook] Property definitions should always be a multiple of 8 bytes! Got size: {prop_data_size}. Returning early");
            return Ok((&[], Vec::new()));
        }

        let prop_count = props.len() / prop_entry_size;
        let mut count = 0;

        let mut props_vec = Vec::new();

        let prop_embedded = [
            PropertyType::Int16,
            PropertyType::Int32,
            PropertyType::Float32,
            PropertyType::ErrorCode,
            PropertyType::Bool,
        ];
        while count < prop_count {
            let (remaining, prop_id) = nom_unsigned_two_bytes(props, Endian::Le)?;
            let (remaining, prop_type_num) = nom_unsigned_two_bytes(remaining, Endian::Le)?;
            let (remaining, value_reference) = nom_unsigned_four_bytes(remaining, Endian::Le)?;

            let name = property_id_to_name(&format!("0x{:04x?}_0x{:04x?}", prop_id, prop_type_num));
            props = remaining;
            count += 1;

            let mut prop = PropertyContext {
                name,
                property_type: get_property_type(&prop_type_num),
                prop_id,
                property_number: prop_type_num,
                reference: value_reference,
                value: Value::Null,
            };

            // If the property value is less than 4 bytes then the value is stored with the definition
            if prop_embedded.contains(&prop.property_type) && prop.reference != 0 {
                prop.value = serde_json::to_value(value_reference).unwrap_or(Value::Null);
            }

            props_vec.push(prop);
        }

        for prop in props_vec.iter_mut() {
            if prop.value != Value::Null || prop.reference == 0 {
                continue;
            }

            let (block_index, map_start) = get_map_offset(&prop.reference);
            if let Some(block_data) = all_blocks.get(block_index as usize) {
                let max_heap_size = 3580;

                if prop.reference > max_heap_size && prop.value == Value::Null {
                    let desc_result =
                        self.get_large_data(ntfs_file, block_descriptors, &prop.reference);
                    let desc_blocks = match desc_result {
                        Ok(result) => result,
                        Err(_err) => {
                            error!("[outlook] Failed to parse descriptors for large data");
                            return Err(nom::Err::Failure(nom::error::Error::new(
                                &[],
                                ErrorKind::Fail,
                            )));
                        }
                    };
                    if !desc_blocks.is_empty() {
                        let all_desc = desc_blocks.concat();
                        // Concat the descriptor data to get the entire property data
                        let prop_result = get_property_data(
                            &all_desc,
                            &prop.property_type,
                            &prop.reference,
                            &true,
                        );

                        let prop_value = match prop_result {
                            Ok((_, result)) => result,
                            Err(_err) => {
                                error!("[outlook] Failed to parse property data from descriptor blocks");
                                continue;
                            }
                        };
                        prop.value = prop_value;

                        continue;
                    }
                    // If we don't have data, then fallback to normal block data below
                }

                let prop_result =
                    get_property_data(block_data, &prop.property_type, &map_start, &false);
                let prop_value = match prop_result {
                    Ok((_, result)) => result,
                    Err(_err) => {
                        error!("[outlook] Failed to parse property data");
                        continue;
                    }
                };
                prop.value = prop_value;
            }
        }

        Ok((&[], props_vec))
    }

    /// If data is too large to fit in the Heap Btree. We have to get the data from the Node Btree
    fn get_large_data(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        block_descriptors: &BTreeMap<u64, DescriptorData>,
        reference: &u32,
    ) -> Result<Vec<Vec<u8>>, OutlookError> {
        let key = (reference >> 5) & 0x07ffffff;
        if let Some(value) = block_descriptors.get(&(key as u64)) {
            let mut leaf_block = LeafBlockData {
                block_type: BlockType::Internal,
                index_id: 0,
                index: 0,
                block_offset: 0,
                size: 0,
                total_size: 0,
                reference_count: 0,
            };
            let mut leaf_descriptor = None;
            for block_tree in &self.block_btree {
                if let Some(block_data) = block_tree.get(&value.block_data_id) {
                    leaf_block = *block_data;

                    if value.block_descriptor_id == 0 {
                        break;
                    }
                }
                if let Some(block_data) = block_tree.get(&value.block_descriptor_id) {
                    leaf_descriptor = Some(*block_data);
                }

                if leaf_descriptor.is_none() && leaf_block.size != 0 {
                    break;
                }
            }
            let value = self.get_block_data(ntfs_file, &leaf_block, leaf_descriptor.as_ref())?;
            return Ok(value.data);
        }

        Ok(Vec::new())
    }
}

/// Parse and get property data
pub(crate) fn get_property_data<'a>(
    data: &'a [u8],
    prop_type: &PropertyType,
    reference: &u32,
    is_large: &bool,
) -> nom::IResult<&'a [u8], Value> {
    let value_data = if *is_large {
        data
    } else {
        // Get the allocation map start
        let (_, allocation_start) = nom_unsigned_two_bytes(data, Endian::Le)?;
        // Jump to the allocation map
        let (allocation, _) = take(allocation_start)(data)?;
        // Skip the allocation count. We do not need it
        let (input, _) = nom_unsigned_four_bytes(allocation, Endian::Le)?;

        if *reference as usize > input.len() {
            return Ok((&[], Value::Null));
        }
        // Jump to the start of our value
        let (value_start, _) = take(*reference)(input)?;
        let (input, value_start) = nom_unsigned_two_bytes(value_start, Endian::Le)?;

        let (_, value_end) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let value_size = value_end - value_start;
        let (input, _) = take(value_start)(data)?;
        let (_, value_data) = take(value_size)(input)?;
        value_data
    };

    extract_property_value(value_data, prop_type)
}

/// Extract the property data into a value
pub(crate) fn extract_property_value<'a>(
    value_data: &'a [u8],
    prop_type: &PropertyType,
) -> nom::IResult<&'a [u8], Value> {
    let mut value = Value::Null;

    match prop_type {
        PropertyType::Int16 => {
            let (_, prop_value) = nom_unsigned_two_bytes(value_data, Endian::Le)?;
            value = serde_json::to_value(prop_value).unwrap_or_default();
        }
        PropertyType::Int32 => {
            let (_, prop_value) = nom_unsigned_four_bytes(value_data, Endian::Le)?;
            value = serde_json::to_value(prop_value).unwrap_or_default();
        }
        PropertyType::Float32 => {
            let (_, float_data) = take(size_of::<u32>())(value_data)?;
            let (_, prop_value) = le_f32(float_data)?;
            value = serde_json::to_value(prop_value).unwrap_or_default();
        }
        PropertyType::Float64 => {
            let (_, float_data) = take(size_of::<u64>())(value_data)?;
            let (_, prop_value) = le_f64(float_data)?;
            value = serde_json::to_value(prop_value).unwrap_or_default();
        }
        PropertyType::FloatTime => {
            // Supposdly this is OLE Time?
            let (_, float_data) = take(size_of::<u64>())(value_data)?;
            let (_, float_value) = le_f64(float_data)?;
            let oletime = ole_automationtime_to_unixepoch(&float_value);
            value = serde_json::to_value(unixepoch_to_iso(&oletime)).unwrap_or_default();
        }
        PropertyType::ErrorCode => {
            // In future we could perhaps translate this to proper error string
            // https://github.com/libyal/libfmapi/blob/main/documentation/MAPI%20definitions.asciidoc#9-error-values-scode
            let (_, prop_value) = nom_unsigned_four_bytes(value_data, Endian::Le)?;
            value = serde_json::to_value(prop_value).unwrap_or_default();
        }
        PropertyType::Bool => {
            let (_, prop_value) = nom_unsigned_one_byte(value_data, Endian::Le)?;
            let prop_bool = prop_value != 0;
            value = serde_json::to_value(prop_bool).unwrap_or_default();
        }
        PropertyType::Int64 => {
            let (_, prop_value) = nom_unsigned_eight_bytes(value_data, Endian::Le)?;
            value = serde_json::to_value(prop_value).unwrap_or_default();
        }
        PropertyType::String
        | PropertyType::MultiString
        | PropertyType::String8
        | PropertyType::MultiString8 => {
            // Strings can either be UTF8 or UTF16 :/
            value = match prop_type {
                PropertyType::String | PropertyType::String8 => {
                    serde_json::to_value(extract_ascii_utf16_string(value_data)).unwrap_or_default()
                }
                PropertyType::MultiString | PropertyType::MultiString8 => {
                    let (mut input, string_count) =
                        nom_unsigned_four_bytes(value_data, Endian::Le)?;
                    let mut count = 0;
                    let mut offsets = Vec::new();
                    while count < string_count {
                        let (remaining, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
                        offsets.push(offset);
                        input = remaining;
                        count += 1;
                    }

                    let mut strings = Vec::new();
                    let mut peek_offsets = offsets.iter().peekable();
                    while let Some(offset) = peek_offsets.next() {
                        let (string_start, _) = take(*offset)(value_data)?;
                        if let Some(next_value) = peek_offsets.peek() {
                            let string_len = *next_value - offset;
                            let (_, final_string) = take(string_len)(string_start)?;
                            let string = extract_ascii_utf16_string(final_string);
                            strings.push(string);
                            continue;
                        }

                        let string = extract_ascii_utf16_string(string_start);
                        strings.push(string);
                    }

                    serde_json::to_value(strings).unwrap_or_default()
                }
                _ => serde_json::to_value(format!("Non string property type. Got {prop_type:?}"))
                    .unwrap_or_default(),
            };
        }
        PropertyType::Time => {
            let (_, prop_value) = nom_unsigned_eight_bytes(value_data, Endian::Le)?;
            let timestamp = filetime_to_unixepoch(&prop_value);
            value = serde_json::to_value(unixepoch_to_iso(&timestamp)).unwrap_or_default();
        }
        PropertyType::Guid => {
            let string_value = format_guid_le_bytes(value_data);
            value = serde_json::to_value(string_value).unwrap_or_default();
        }
        PropertyType::MultiInt16 => {
            let int_count = value_data.len() / 2;
            let mut remaining = value_data;
            let mut count = 0;

            let mut int_values = Vec::new();
            while count < int_count {
                let (input, int_value) = nom_unsigned_two_bytes(remaining, Endian::Le)?;
                remaining = input;
                int_values.push(int_value);
                count += 1;
            }

            value = serde_json::to_value(&int_values).unwrap_or_default();
        }
        PropertyType::MultiInt32 => {
            let int_count = value_data.len() / 4;
            let mut remaining = value_data;
            let mut count = 0;

            let mut int_values = Vec::new();
            while count < int_count {
                let (input, int_value) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
                remaining = input;
                int_values.push(int_value);
                count += 1;
            }

            value = serde_json::to_value(&int_values).unwrap_or_default();
        }
        PropertyType::MultiFloat32 => {
            let int_count = value_data.len() / 4;
            let mut remaining = value_data;
            let mut count = 0;

            let mut int_values = Vec::new();
            while count < int_count {
                let (input, float_data) = take(size_of::<u32>())(remaining)?;
                let (_, prop_value) = le_f32(float_data)?;
                remaining = input;
                int_values.push(prop_value);
                count += 1;
            }

            value = serde_json::to_value(&int_values).unwrap_or_default();
        }
        PropertyType::MultiFloat64 => {
            let int_count = value_data.len() / 8;
            let mut remaining = value_data;
            let mut count = 0;

            let mut int_values = Vec::new();
            while count < int_count {
                let (input, float_data) = take(size_of::<u64>())(remaining)?;
                let (_, prop_value) = le_f64(float_data)?;
                remaining = input;
                int_values.push(prop_value);
                count += 1;
            }

            value = serde_json::to_value(&int_values).unwrap_or_default();
        }
        PropertyType::MultiFloatTime => {
            let int_count = value_data.len() / 8;
            let mut remaining = value_data;
            let mut count = 0;

            let mut int_values = Vec::new();

            while count < int_count {
                // Supposedly this is OLE Time?
                let (input, float_data) = take(size_of::<u64>())(remaining)?;
                let (_, float_value) = le_f64(float_data)?;
                remaining = input;
                let oletime = ole_automationtime_to_unixepoch(&float_value);
                int_values.push(unixepoch_to_iso(&oletime));
                count += 1;
            }
            value = serde_json::to_value(int_values).unwrap_or_default();
        }
        PropertyType::MultiInt64 => {
            let int_count = value_data.len() / 8;
            let mut remaining = value_data;
            let mut count = 0;

            let mut int_values = Vec::new();
            while count < int_count {
                let (input, int_value) = nom_unsigned_eight_bytes(remaining, Endian::Le)?;
                remaining = input;
                int_values.push(int_value);
                count += 1;
            }

            value = serde_json::to_value(&int_values).unwrap_or_default();
        }
        PropertyType::MultiTime => {
            let int_count = value_data.len() / 8;
            let mut remaining = value_data;
            let mut count = 0;

            let mut int_values = Vec::new();
            while count < int_count {
                let (input, prop_value) = nom_unsigned_eight_bytes(remaining, Endian::Le)?;
                let timestamp = filetime_to_unixepoch(&prop_value);

                remaining = input;
                int_values.push(unixepoch_to_iso(&timestamp));
                count += 1;
            }
            value = serde_json::to_value(int_values).unwrap_or_default();
        }
        PropertyType::MultiGuid => {
            warn!("[outlook] Got multi-guid property. Attempting to parse");
            let guid_size = 16;
            let guid_count = value_data.len() / guid_size;
            let mut remaining = value_data;
            let mut count = 0;

            let mut int_values = Vec::new();
            while count < guid_count {
                let (input, guid_value) = take(guid_size)(remaining)?;
                remaining = input;
                int_values.push(format_guid_le_bytes(guid_value));
                count += 1;
            }

            value = serde_json::to_value(&int_values).unwrap_or_default();
        }
        PropertyType::MultiBinary => {
            let (offset_start, bin_count) = nom_unsigned_four_bytes(value_data, Endian::Le)?;
            let empty = 0;
            if bin_count != empty {
                let mut remaining = offset_start;
                let mut offsets = Vec::new();
                let mut count = 0;
                while count < bin_count {
                    let (input, offset) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
                    remaining = input;
                    offsets.push(offset);
                    count += 1;
                }

                let mut peek_offsets = offsets.iter().peekable();
                let mut binary_values = Vec::new();
                while let Some(offset) = peek_offsets.next() {
                    let (bin_start, _) = take(*offset)(value_data)?;
                    if let Some(next_value) = peek_offsets.peek() {
                        let bin_len = *next_value - offset;
                        if bin_len == empty {
                            // If length is zero, value is null
                            continue;
                        }
                        let (_, final_bin) = take(bin_len)(bin_start)?;
                        let string = base64_encode_standard(final_bin);
                        binary_values.push(string);
                        continue;
                    }
                }
                value = serde_json::to_value(&binary_values).unwrap_or_default();
            }
        }
        // We are already NULL. Unspecified means the value type does not matter
        PropertyType::Null | PropertyType::Unspecified => {}
        // These properties have not been observed in the test data we have. Base64 encode for now. In future this can be added once we have test data
        PropertyType::Unknown
        | PropertyType::Currency
        | PropertyType::MultiCurrency
        | PropertyType::RuleAction
        | PropertyType::Object
        | PropertyType::Restriction
        | PropertyType::Binary
        | PropertyType::ServerId => {
            value = serde_json::to_value(base64_encode_standard(value_data)).unwrap_or_default();
        }
    };

    Ok((value_data, value))
}

/// Get offsets in allocation map from reference
pub(crate) fn get_map_offset(reference: &u32) -> (u32, u32) {
    let unicode_4k = 19;
    let block_index = reference >> unicode_4k;

    let adjust = 0x07ffe0;
    let value_lower_bits = 5;
    let map_start = ((reference & adjust) >> value_lower_bits) - 1;

    // The Allocation map array is in pairs (start, end)
    let pairs = 2;
    (block_index, map_start * pairs)
}

#[cfg(test)]
mod tests {
    use super::{get_map_offset, get_property_data};
    use crate::{
        artifacts::os::windows::outlook::{
            blocks::block::{Block, BlockValue},
            header::FormatType,
            helper::{OutlookReader, OutlookReaderAction},
            pages::btree::{BlockType, LeafBlockData},
            tables::{
                context::PropertyType,
                properties::PropertyName,
                property::{extract_property_value, OutlookPropertyContext},
            },
        },
        filesystem::files::file_reader,
    };
    use serde_json::Value;
    use std::{collections::BTreeMap, io::BufReader, path::PathBuf};

    #[test]
    fn test_parse_property_context_root_folder() {
        let test = [
            70, 2, 236, 188, 32, 0, 0, 0, 0, 0, 0, 0, 181, 2, 6, 0, 64, 0, 0, 0, 1, 48, 31, 0, 0,
            0, 0, 0, 4, 48, 31, 0, 0, 0, 0, 0, 7, 48, 64, 0, 128, 0, 0, 0, 8, 48, 64, 0, 96, 0, 0,
            0, 2, 54, 3, 0, 0, 0, 0, 0, 3, 54, 3, 0, 0, 0, 0, 0, 10, 54, 11, 0, 1, 0, 0, 0, 228,
            63, 11, 0, 0, 0, 0, 0, 229, 63, 11, 0, 0, 0, 0, 0, 20, 102, 2, 1, 160, 0, 0, 0, 56,
            102, 3, 0, 2, 0, 0, 0, 57, 102, 3, 0, 251, 5, 0, 0, 112, 189, 150, 244, 111, 225, 218,
            1, 112, 189, 150, 244, 111, 225, 218, 1, 70, 53, 70, 86, 3, 0, 0, 0, 177, 0, 0, 0, 106,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 142, 0, 0, 0, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 27,
            1, 0, 0, 68, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 104, 0, 0, 0, 8, 0, 0, 0, 94, 178, 150, 180, 131, 77, 40, 66, 134, 11, 232, 66,
            98, 69, 158, 194, 6, 0, 0, 0, 0, 1, 12, 0, 3, 0, 0, 0, 0, 0, 0, 0, 94, 178, 150, 180,
            131, 77, 40, 66, 134, 11, 232, 66, 98, 69, 158, 194, 82, 0, 0, 0, 0, 0, 1, 0, 3, 34,
            183, 166, 197, 0, 94, 178, 150, 180, 131, 77, 40, 66, 134, 11, 232, 66, 98, 69, 158,
            194, 82, 0, 0, 0, 0, 0, 1, 0, 3, 34, 183, 166, 197, 0, 91, 220, 80, 80, 0, 47, 111, 61,
            70, 105, 114, 115, 116, 32, 79, 114, 103, 97, 110, 105, 122, 97, 116, 105, 111, 110,
            47, 111, 117, 61, 69, 120, 99, 104, 97, 110, 103, 101, 32, 65, 100, 109, 105, 110, 105,
            115, 116, 114, 97, 116, 105, 118, 101, 32, 71, 114, 111, 117, 112, 40, 70, 89, 68, 73,
            66, 79, 72, 70, 50, 51, 83, 80, 68, 76, 84, 41, 47, 99, 110, 61, 82, 101, 99, 105, 112,
            105, 101, 110, 116, 115, 47, 99, 110, 61, 48, 48, 48, 51, 66, 70, 70, 68, 51, 57, 56,
            69, 69, 66, 48, 49, 0, 94, 178, 150, 180, 131, 77, 40, 66, 134, 11, 232, 66, 98, 69,
            158, 194, 1, 0, 1, 0, 3, 0, 0, 1, 82, 9, 18, 66, 27, 4, 66, 39, 253, 66, 77, 193, 66,
            92, 23, 80, 3, 133, 158, 143, 82, 134, 135, 80, 80, 3, 3, 20, 32, 1, 30, 82, 184, 187,
            80, 1, 91, 82, 219, 220, 80, 80, 80, 0, 23, 80, 3, 133, 158, 143, 82, 134, 135, 80, 80,
            3, 3, 20, 32, 1, 30, 82, 184, 187, 80, 1, 91, 82, 219, 220, 80, 80, 80, 0, 94, 178,
            150, 180, 131, 77, 40, 66, 134, 11, 232, 66, 98, 69, 158, 194, 1, 0, 1, 0, 3, 0, 0, 1,
            82, 9, 18, 66, 27, 4, 66, 39, 253, 66, 77, 193, 66, 92, 23, 80, 3, 133, 158, 143, 82,
            134, 135, 80, 80, 3, 3, 20, 32, 1, 30, 82, 184, 187, 80, 1, 91, 82, 219, 220, 80, 80,
            80, 0, 0, 5, 0, 0, 0, 12, 0, 20, 0, 116, 0, 124, 0, 132, 0, 69, 2,
        ];

        // We dont need an OST file for this test
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/node.raw");
        let reader = file_reader(test_location.to_str().unwrap()).unwrap();
        let buf_reader = BufReader::new(reader);

        let mut outlook_reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unicode64_4k,
            size: 4096,
        };
        let block = BlockValue {
            block_type: Block::Xblock,
            data: vec![test.to_vec()],
            descriptors: BTreeMap::new(),
        };
        let result = outlook_reader
            .parse_property_context(None, &block.data, &block.descriptors)
            .unwrap();

        // let (_, result) = parse_property_context(&test).unwrap();
        assert_eq!(result[2].property_type, PropertyType::Time);
        assert_eq!(result[2].name, vec![PropertyName::PidTagCreationTime]);
        assert_eq!(
            result[2].value.as_str().unwrap(),
            "2024-07-29T04:29:52.000Z"
        );

        assert_eq!(result[9].property_type, PropertyType::Binary);
        assert_eq!(result[9].name, vec![PropertyName::Unknown]);
        assert_eq!(result[9].value.as_str().unwrap(), "RjVGVgMAAACxAAAAagAAAAAAAAAAAAAAjgAAAB4AAAAAAAAAAAAAABsBAABEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAaAAAAAgAAABespa0g00oQoYL6EJiRZ7CBgAAAAABDAADAAAAAAAAAF6ylrSDTShChgvoQmJFnsJSAAAAAAABAAMit6bFAF6ylrSDTShChgvoQmJFnsJSAAAAAAABAAMit6bFAFvcUFAAL289Rmlyc3QgT3JnYW5pemF0aW9uL291PUV4Y2hhbmdlIEFkbWluaXN0cmF0aXZlIEdyb3VwKEZZRElCT0hGMjNTUERMVCkvY249UmVjaXBpZW50cy9jbj0wMDAzQkZGRDM5OEVFQjAxAF6ylrSDTShChgvoQmJFnsIBAAEAAwAAAVIJEkIbBEIn/UJNwUJcF1ADhZ6PUoaHUFADAxQgAR5SuLtQAVtS29xQUFAAF1ADhZ6PUoaHUFADAxQgAR5SuLtQAVtS29xQUFAAXrKWtINNKEKGC+hCYkWewgEAAQADAAABUgkSQhsEQif9Qk3BQlwXUAOFno9ShodQUAMDFCABHlK4u1ABW1Lb3FBQUAA=");
    }

    #[test]
    fn test_get_property_data() {
        let test = [
            70, 2, 236, 188, 32, 0, 0, 0, 0, 0, 0, 0, 181, 2, 6, 0, 64, 0, 0, 0, 1, 48, 31, 0, 0,
            0, 0, 0, 4, 48, 31, 0, 0, 0, 0, 0, 7, 48, 64, 0, 128, 0, 0, 0, 8, 48, 64, 0, 96, 0, 0,
            0, 2, 54, 3, 0, 0, 0, 0, 0, 3, 54, 3, 0, 0, 0, 0, 0, 10, 54, 11, 0, 1, 0, 0, 0, 228,
            63, 11, 0, 0, 0, 0, 0, 229, 63, 11, 0, 0, 0, 0, 0, 20, 102, 2, 1, 160, 0, 0, 0, 56,
            102, 3, 0, 2, 0, 0, 0, 57, 102, 3, 0, 251, 5, 0, 0, 112, 189, 150, 244, 111, 225, 218,
            1, 112, 189, 150, 244, 111, 225, 218, 1, 70, 53, 70, 86, 3, 0, 0, 0, 177, 0, 0, 0, 106,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 142, 0, 0, 0, 30, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 27,
            1, 0, 0, 68, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 104, 0, 0, 0, 8, 0, 0, 0, 94, 178, 150, 180, 131, 77, 40, 66, 134, 11, 232, 66,
            98, 69, 158, 194, 6, 0, 0, 0, 0, 1, 12, 0, 3, 0, 0, 0, 0, 0, 0, 0, 94, 178, 150, 180,
            131, 77, 40, 66, 134, 11, 232, 66, 98, 69, 158, 194, 82, 0, 0, 0, 0, 0, 1, 0, 3, 34,
            183, 166, 197, 0, 94, 178, 150, 180, 131, 77, 40, 66, 134, 11, 232, 66, 98, 69, 158,
            194, 82, 0, 0, 0, 0, 0, 1, 0, 3, 34, 183, 166, 197, 0, 91, 220, 80, 80, 0, 47, 111, 61,
            70, 105, 114, 115, 116, 32, 79, 114, 103, 97, 110, 105, 122, 97, 116, 105, 111, 110,
            47, 111, 117, 61, 69, 120, 99, 104, 97, 110, 103, 101, 32, 65, 100, 109, 105, 110, 105,
            115, 116, 114, 97, 116, 105, 118, 101, 32, 71, 114, 111, 117, 112, 40, 70, 89, 68, 73,
            66, 79, 72, 70, 50, 51, 83, 80, 68, 76, 84, 41, 47, 99, 110, 61, 82, 101, 99, 105, 112,
            105, 101, 110, 116, 115, 47, 99, 110, 61, 48, 48, 48, 51, 66, 70, 70, 68, 51, 57, 56,
            69, 69, 66, 48, 49, 0, 94, 178, 150, 180, 131, 77, 40, 66, 134, 11, 232, 66, 98, 69,
            158, 194, 1, 0, 1, 0, 3, 0, 0, 1, 82, 9, 18, 66, 27, 4, 66, 39, 253, 66, 77, 193, 66,
            92, 23, 80, 3, 133, 158, 143, 82, 134, 135, 80, 80, 3, 3, 20, 32, 1, 30, 82, 184, 187,
            80, 1, 91, 82, 219, 220, 80, 80, 80, 0, 23, 80, 3, 133, 158, 143, 82, 134, 135, 80, 80,
            3, 3, 20, 32, 1, 30, 82, 184, 187, 80, 1, 91, 82, 219, 220, 80, 80, 80, 0, 94, 178,
            150, 180, 131, 77, 40, 66, 134, 11, 232, 66, 98, 69, 158, 194, 1, 0, 1, 0, 3, 0, 0, 1,
            82, 9, 18, 66, 27, 4, 66, 39, 253, 66, 77, 193, 66, 92, 23, 80, 3, 133, 158, 143, 82,
            134, 135, 80, 80, 3, 3, 20, 32, 1, 30, 82, 184, 187, 80, 1, 91, 82, 219, 220, 80, 80,
            80, 0, 0, 5, 0, 0, 0, 12, 0, 20, 0, 116, 0, 124, 0, 132, 0, 69, 2,
        ];
        let (_, value) = get_property_data(&test, &PropertyType::Time, &4, &false).unwrap();
        assert_eq!(value.as_str().unwrap(), "2024-07-29T04:29:52.000Z");
    }

    #[test]
    fn test_parse_property_context_message_store() {
        let test = [
            174, 1, 236, 188, 32, 0, 0, 0, 0, 0, 0, 0, 181, 2, 6, 0, 64, 0, 0, 0, 92, 14, 11, 0, 1,
            0, 0, 0, 249, 15, 2, 1, 96, 0, 0, 0, 1, 48, 31, 0, 0, 0, 0, 0, 22, 52, 2, 1, 32, 1, 0,
            0, 21, 102, 72, 0, 160, 0, 0, 0, 31, 102, 20, 0, 0, 1, 0, 0, 32, 102, 3, 0, 249, 1, 0,
            0, 51, 102, 11, 0, 1, 0, 0, 0, 109, 102, 3, 0, 0, 140, 0, 0, 250, 102, 3, 0, 17, 0, 14,
            0, 252, 102, 3, 0, 62, 175, 24, 0, 255, 103, 3, 0, 255, 255, 255, 255, 4, 124, 2, 1,
            192, 0, 0, 0, 6, 124, 31, 16, 64, 1, 0, 0, 7, 124, 2, 1, 128, 0, 0, 0, 12, 124, 3, 0,
            0, 0, 0, 0, 13, 124, 20, 0, 224, 0, 0, 0, 17, 124, 11, 0, 1, 0, 0, 0, 19, 124, 3, 0, 4,
            55, 18, 0, 13, 121, 253, 85, 247, 74, 143, 77, 141, 121, 129, 146, 72, 127, 210, 0, 1,
            0, 0, 0, 186, 86, 57, 234, 168, 210, 22, 74, 160, 69, 90, 22, 243, 172, 249, 176, 1, 8,
            0, 0, 0, 252, 0, 0, 0, 0, 0, 0, 94, 178, 150, 180, 131, 77, 40, 66, 134, 11, 232, 66,
            98, 69, 158, 194, 0, 3, 34, 185, 231, 130, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 91, 215, 45, 167, 52, 215, 220, 68, 175, 222, 60, 208,
            93, 32, 138, 165, 70, 100, 212, 225, 117, 185, 224, 64, 185, 193, 109, 232, 93, 23, 22,
            10, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 3, 34, 185, 229, 137, 0, 0, 0, 0, 91,
            215, 45, 167, 52, 215, 220, 68, 175, 222, 60, 208, 93, 32, 138, 165, 1, 0, 94, 178,
            150, 180, 131, 77, 40, 66, 134, 11, 232, 66, 98, 69, 158, 194, 0, 3, 34, 185, 229, 131,
            0, 0, 1, 0, 0, 0, 8, 0, 0, 0, 92, 0, 79, 0, 102, 0, 102, 0, 108, 0, 105, 0, 110, 0,
            101, 0, 32, 0, 71, 0, 108, 0, 111, 0, 98, 0, 97, 0, 108, 0, 32, 0, 65, 0, 100, 0, 100,
            0, 114, 0, 101, 0, 115, 0, 115, 0, 32, 0, 76, 0, 105, 0, 115, 0, 116, 0, 10, 0, 0, 0,
            12, 0, 20, 0, 172, 0, 188, 0, 12, 1, 28, 1, 48, 1, 56, 1, 64, 1, 110, 1, 174, 1,
        ];

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/node.raw");
        let reader = file_reader(test_location.to_str().unwrap()).unwrap();
        let buf_reader = BufReader::new(reader);

        let mut outlook_reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unicode64_4k,
            size: 4096,
        };
        let block = BlockValue {
            block_type: Block::Xblock,
            data: vec![test.to_vec()],
            descriptors: BTreeMap::new(),
        };
        let store = outlook_reader
            .parse_property_context(None, &block.data, &block.descriptors)
            .unwrap();

        assert_eq!(store.len(), 19);
        assert_eq!(store[3].name, vec![PropertyName::Unknown]);
        assert_eq!(store[3].property_type, PropertyType::Binary);
        assert_eq!(
            store[3].value.as_str().unwrap(),
            "AAAAAFvXLac019xEr9480F0giqUBAF6ylrSDTShChgvoQmJFnsIAAyK55YMAAA=="
        );

        assert_eq!(store[13].name, vec![PropertyName::Unknown]);
        assert_eq!(store[13].property_type, PropertyType::MultiString);
        assert_eq!(
            store[13].value.as_array().unwrap(),
            &vec![serde_json::to_value("\\Offline Global Address List").unwrap()]
        );
    }

    #[test]
    fn test_parse_property_context_name_to_id_map() {
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
        let results = outlook_reader
            .parse_property_context(None, &block_value.data, &block_value.descriptors)
            .unwrap();
        assert_eq!(results[1].value.as_str().unwrap().len(), 940);
    }

    #[test]
    fn test_get_map_offset() {
        let (block_index, map_start) = get_map_offset(&96);
        assert_eq!(block_index, 0);
        assert_eq!(map_start, 4);
    }

    #[test]
    fn test_get_property_data_multi_binary() {
        let test = [
            86, 2, 236, 188, 32, 0, 0, 0, 0, 0, 0, 0, 181, 2, 6, 0, 64, 0, 0, 0, 1, 48, 31, 0, 160,
            0, 0, 0, 4, 48, 31, 0, 0, 0, 0, 0, 7, 48, 64, 0, 128, 0, 0, 0, 8, 48, 64, 0, 96, 0, 0,
            0, 2, 54, 3, 0, 0, 0, 0, 0, 3, 54, 3, 0, 0, 0, 0, 0, 10, 54, 11, 0, 1, 0, 0, 0, 213,
            54, 2, 1, 32, 1, 0, 0, 215, 54, 2, 1, 128, 1, 0, 0, 216, 54, 2, 17, 96, 1, 0, 0, 217,
            54, 2, 1, 64, 1, 0, 0, 228, 63, 11, 0, 0, 0, 0, 0, 229, 63, 11, 0, 0, 0, 0, 0, 226,
            101, 2, 1, 224, 0, 0, 0, 227, 101, 2, 1, 0, 1, 0, 0, 20, 102, 2, 1, 0, 0, 0, 0, 56,
            102, 3, 0, 11, 0, 0, 0, 57, 102, 3, 0, 1, 0, 0, 0, 244, 103, 20, 0, 192, 0, 0, 0, 1,
            104, 3, 0, 2, 0, 0, 0, 16, 128, 3, 0, 2, 0, 0, 0, 48, 255, 41, 22, 190, 4, 219, 1, 16,
            190, 42, 18, 190, 4, 219, 1, 82, 0, 111, 0, 111, 0, 116, 0, 32, 0, 45, 0, 32, 0, 77, 0,
            97, 0, 105, 0, 108, 0, 98, 0, 111, 0, 120, 0, 0, 0, 0, 0, 0, 0, 0, 5, 122, 180, 193,
            196, 149, 18, 19, 77, 190, 54, 248, 71, 119, 70, 98, 170, 0, 0, 8, 37, 20, 122, 180,
            193, 196, 149, 18, 19, 77, 190, 54, 248, 71, 119, 70, 98, 170, 0, 0, 8, 37, 0, 0, 0, 0,
            231, 133, 221, 18, 152, 103, 116, 76, 165, 251, 197, 110, 145, 99, 30, 98, 1, 0, 195,
            39, 9, 165, 214, 162, 153, 77, 151, 84, 182, 92, 93, 27, 160, 198, 0, 0, 0, 0, 2, 0, 0,
            0, 2, 128, 50, 0, 1, 0, 46, 0, 0, 0, 0, 0, 231, 133, 221, 18, 152, 103, 116, 76, 165,
            251, 197, 110, 145, 99, 30, 98, 1, 0, 195, 39, 9, 165, 214, 162, 153, 77, 151, 84, 182,
            92, 93, 27, 160, 198, 0, 0, 0, 0, 2, 1, 0, 0, 0, 0, 0, 0, 6, 0, 0, 0, 28, 0, 0, 0, 28,
            0, 0, 0, 74, 0, 0, 0, 120, 0, 0, 0, 120, 0, 0, 0, 166, 0, 0, 0, 0, 0, 0, 0, 231, 133,
            221, 18, 152, 103, 116, 76, 165, 251, 197, 110, 145, 99, 30, 98, 1, 0, 195, 39, 9, 165,
            214, 162, 153, 77, 151, 84, 182, 92, 93, 27, 160, 198, 0, 0, 0, 0, 2, 2, 0, 0, 0, 0, 0,
            0, 231, 133, 221, 18, 152, 103, 116, 76, 165, 251, 197, 110, 145, 99, 30, 98, 1, 0,
            195, 39, 9, 165, 214, 162, 153, 77, 151, 84, 182, 92, 93, 27, 160, 198, 0, 0, 0, 0, 2,
            3, 0, 0, 0, 0, 0, 0, 231, 133, 221, 18, 152, 103, 116, 76, 165, 251, 197, 110, 145, 99,
            30, 98, 1, 0, 52, 173, 118, 24, 117, 145, 151, 77, 191, 108, 79, 118, 149, 28, 135, 58,
            1, 0, 3, 0, 0, 0, 0, 0, 0, 0, 0, 0, 231, 133, 221, 18, 152, 103, 116, 76, 165, 251,
            197, 110, 145, 99, 30, 98, 1, 0, 52, 173, 118, 24, 117, 145, 151, 77, 191, 108, 79,
            118, 149, 28, 135, 58, 1, 0, 1, 0, 0, 0, 0, 0, 1, 12, 0, 0, 0, 12, 0, 20, 0, 188, 0,
            196, 0, 204, 0, 232, 0, 240, 0, 4, 1, 25, 1, 71, 1, 129, 1, 39, 2, 85, 2,
        ];

        let (_, result) =
            get_property_data(&test, &PropertyType::MultiBinary, &20, &false).unwrap();

        assert_eq!(
            result.as_array().unwrap(),
            &vec![
                Value::String(
                    "AAAAAOeF3RKYZ3RMpfvFbpFjHmIBAMMnCaXWoplNl1S2XF0boMYAAAAAAgIAAA==".to_string()
                ),
                Value::String(
                    "AAAAAOeF3RKYZ3RMpfvFbpFjHmIBAMMnCaXWoplNl1S2XF0boMYAAAAAAgMAAA==".to_string()
                ),
                Value::String(
                    "AAAAAOeF3RKYZ3RMpfvFbpFjHmIBADStdhh1kZdNv2xPdpUchzoBAAMAAAAAAA==".to_string()
                )
            ]
        );
    }

    #[test]
    fn teast_extract_property_value() {
        let test = [0, 1, 8, 16, 0, 3, 0, 4, 1];
        let (_, result) = extract_property_value(&test, &PropertyType::Currency).unwrap();
        assert_eq!(result.as_str().unwrap(), "AAEIEAADAAQB");
    }
}
