use super::{
    header::HeapNode,
    properties::{property_id_to_name, PropertyName},
    property::get_property_data,
};
use crate::{
    artifacts::os::windows::outlook::{
        blocks::descriptors::DescriptorData,
        error::OutlookError,
        header::NodeID,
        helper::{OutlookReader, OutlookReaderAction},
        pages::btree::{BlockType, LeafBlockData},
        tables::{
            header::{get_heap_node_id, table_header},
            heap_btree::parse_btree_heap,
            property::get_map_offset,
        },
    },
    utils::{
        encoding::base64_encode_standard,
        nom_helper::{
            nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
            nom_unsigned_two_bytes, Endian,
        },
        time::{filetime_to_unixepoch, unixepoch_to_iso},
        uuid::format_guid_le_bytes,
    },
};
use log::error;
use nom::bytes::complete::take;
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Debug)]
pub(crate) struct TableContext {
    sig: u8,
    number_column_definitions: u8,
    array_end_32bit: u16,
    array_end_16bit: u16,
    array_end_8bit: u16,
    array_end_offset: u16,
    row_index: HeapNode,
    /**Will be found in either Heap BTree or NodeBtree. Depends on `NodeID` value */
    row: HeapNode,
    pub(crate) rows: Vec<Vec<TableRows>>,
}

#[derive(Debug, Clone)]
pub(crate) struct TableRows {
    pub(crate) value: Value,
    pub(crate) column: ColumnDescriptor,
}

#[derive(Debug, Clone)]
pub(crate) struct ColumnDescriptor {
    pub(crate) property_type: PropertyType,
    pub(crate) id: u16,
    pub(crate) property_name: Vec<PropertyName>,
    offset: u16,
    size: u8,
    index: u8,
}

#[derive(Debug, PartialEq, Clone)]
pub(crate) enum PropertyType {
    Int16,
    Int32,
    Float32,
    Float64,
    Currency,
    FloatTime,
    ErrorCode,
    Bool,
    Int64,
    String,
    String8,
    Time,
    Guid,
    ServerId,
    Restriction,
    Binary,
    MultiInt16,
    MultiInt32,
    MultiFloat32,
    MultiFloat64,
    MultiCurrency,
    MultiFloatTime,
    MultiInt64,
    MultiString,
    MultiString8,
    MultiTime,
    MultiGuid,
    MultiBinary,
    Unspecified,
    Null,
    Object,
    RuleAction,
    Unknown,
}

pub(crate) trait OutlookTableContext<T: std::io::Seek + std::io::Read> {
    fn table_info(
        &mut self,
        block_data: &Vec<Vec<u8>>,
        block_descriptors: &BTreeMap<u64, DescriptorData>,
    ) -> Result<TableInfo, OutlookError>;

    fn get_rows(&mut self, info: &TableInfo) -> Result<Vec<Vec<TableRows>>, OutlookError>;
    fn parse_rows<'a>(
        &mut self,
        info: &TableInfo,
        data: &'a [u8],
    ) -> nom::IResult<&'a [u8], Vec<Vec<TableRows>>>;
    fn parse_table_info<'a>(
        &mut self,
        data: &'a [u8],
        all_block: &Vec<Vec<u8>>,
        descriptors: &BTreeMap<u64, DescriptorData>,
    ) -> nom::IResult<&'a [u8], TableInfo>;

    fn get_descriptor_data(
        &mut self,
        descriptor: &DescriptorData,
    ) -> Result<Vec<Vec<u8>>, OutlookError>;
}

#[derive(Debug, Clone)]
pub(crate) struct TableInfo {
    pub(crate) block_data: Vec<Vec<u8>>,
    pub(crate) block_descriptors: BTreeMap<u64, DescriptorData>,
    pub(crate) rows: Vec<u64>,
    pub(crate) columns: Vec<TableRows>,
    pub(crate) include_cols: Vec<PropertyName>,
    pub(crate) row_size: u16,
    pub(crate) map_offset: u16,
    pub(crate) node: HeapNode,
    pub(crate) total_rows: u64,
}

impl<T: std::io::Seek + std::io::Read> OutlookTableContext<T> for OutlookReader<T> {
    fn table_info(
        &mut self,
        block_data: &Vec<Vec<u8>>,
        block_descriptors: &BTreeMap<u64, DescriptorData>,
    ) -> Result<TableInfo, OutlookError> {
        let first_block = block_data.get(0);
        let block = match first_block {
            Some(result) => result,
            None => return Err(OutlookError::NoBlocks),
        };

        let props_result = self.parse_table_info(block, block_data, block_descriptors);
        let table = match props_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[outlook] Could not get table info");
                return Err(OutlookError::TableContext);
            }
        };

        Ok(table)
    }

    fn get_rows(&mut self, info: &TableInfo) -> Result<Vec<Vec<TableRows>>, OutlookError> {
        let first_block = info.block_data.get(0);
        let block = match first_block {
            Some(result) => result,
            None => return Err(OutlookError::NoBlocks),
        };

        let rows_result = self.parse_rows(info, &block);
        let rows = match rows_result {
            Ok((_, result)) => result,
            Err(_err) => {
                error!("[outlook] Could not get table rows");
                return Err(OutlookError::TableContext);
            }
        };
        Ok(rows)
    }

    fn parse_rows<'a>(
        &mut self,
        info: &TableInfo,
        data: &'a [u8],
    ) -> nom::IResult<&'a [u8], Vec<Vec<TableRows>>> {
        let (input, header) = table_header(data)?;
        let (input, heap_btree) = parse_btree_heap(input)?;

        let tree_header_size: u8 = 22;
        let (input, _) = take(tree_header_size)(input)?;

        let mut descriptor_data = Vec::new();
        if info.node.node == NodeID::LocalDescriptors {
            println!("TC Desc: {:?}", info.block_descriptors);
            if let Some(descriptor) = info.block_descriptors.get(&(info.node.index as u64)) {
                descriptor_data = self.get_descriptor_data(descriptor).unwrap();
            }
        }

        let column_size = 8;
        let column_definition_size = column_size * info.columns.len();
        // Now skip column definitions
        let (input, _) = take(column_definition_size)(input)?;

        // Now skip Row name and ID section
        let section_size = 8;
        let size = info.total_rows * section_size;
        let (input, _) = take(size)(input)?;

        if !descriptor_data.is_empty() {
            let (input, rows) = get_row_data(&descriptor_data[0], info).unwrap();

            return Ok((&[], rows));
        }

        let (input, rows) = get_row_data(&input, info)?;

        Ok((input, rows))
    }

    fn parse_table_info<'a>(
        &mut self,
        data: &'a [u8],
        all_block: &Vec<Vec<u8>>,
        descriptors: &BTreeMap<u64, DescriptorData>,
    ) -> nom::IResult<&'a [u8], TableInfo> {
        let (input, header) = table_header(data)?;
        println!("Table context header: {header:?}");
        println!(
            "Allocation table len: {}",
            header.page_map.allocation_table.len()
        );
        let (input, heap_btree) = parse_btree_heap(input)?;
        println!("Table context heap tree: {heap_btree:?}");

        let (input, sig) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, number_column_definitions) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, array_end_32bit) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, array_end_16bit) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, array_end_8bit) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, array_end_offset) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, table_context_index_reference) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let row_index = get_heap_node_id(&table_context_index_reference);
        println!("Row Index HeapNode: {row_index:?}");
        let (input, values_array_index_reference) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let row = get_heap_node_id(&values_array_index_reference);
        println!("Row Heap Node: {row:?}");

        let (input, _padding) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let row_count = get_row_count(&header.page_map.allocation_table);

        let (input, rows) = get_column_definitions(input, &number_column_definitions)?;
        println!("Rows: {}", rows.len());

        let info = TableInfo {
            block_data: all_block.to_vec(),
            block_descriptors: descriptors.clone(),
            rows: Vec::new(),
            columns: rows,
            include_cols: Vec::new(),
            row_size: array_end_offset,
            map_offset: header.page_map_offset,
            total_rows: row_count,
            node: row,
        };

        Ok((input, info))
    }

    fn get_descriptor_data(
        &mut self,
        descriptor: &DescriptorData,
    ) -> Result<Vec<Vec<u8>>, OutlookError> {
        let mut leaf_block = LeafBlockData {
            block_type: BlockType::Internal,
            index_id: 0,
            index: 0,
            block_offset: 0,
            size: 0,
            total_size: 0,
            reference_count: 0,
        };
        println!("desc: {descriptor:?}");
        let mut leaf_descriptor = None;
        for block_tree in &self.block_btree {
            if let Some(block_data) = block_tree.get(&descriptor.block_data_id) {
                leaf_block = block_data.clone();

                if descriptor.block_descriptor_id == 0 {
                    break;
                }
            }
            if let Some(block_data) = block_tree.get(&descriptor.block_descriptor_id) {
                leaf_descriptor = Some(block_data.clone());
            }

            if leaf_descriptor.is_none() && leaf_block.size != 0 {
                break;
            }
        }
        let value = self.get_block_data(None, &leaf_block, leaf_descriptor.as_ref())?;
        return Ok(value.data);
    }
}

fn get_row_count(map: &[u16]) -> u64 {
    if map.len() < 4 {
        // There are no rows
        return 0;
    }

    let row_start = map[2];
    let row_end = map[3];
    let rows = row_end - row_start;

    let row_size = 8;
    if rows % row_size != 0 {
        panic!("rows should always be a multiple of 8 bytes?! {rows}");
    }

    let count = rows / row_size;
    count as u64
}

fn get_row_data<'a>(
    data: &'a [u8],
    info: &TableInfo,
) -> nom::IResult<&'a [u8], Vec<Vec<TableRows>>> {
    let mut rows = Vec::new();
    // Get the rows we want
    for entry in &info.rows {
        // Go to the start of the row
        let (row_start, _) = take(entry * info.row_size as u64)(data)?;
        let (_, row_data) = take(info.row_size)(row_start)?;

        // Give each row column info
        let mut col = info.columns.clone();
        for column in col.iter_mut() {
            let (_, value) = parse_row_data(
                &info.block_data,
                row_data,
                &column.column.property_type,
                &info.map_offset,
                &column.column.offset,
                &column.column.size,
            )?;

            column.value = value;
        }

        rows.push(col);
    }

    Ok((data, rows))
}

fn parse_row_data<'a>(
    all_blocks: &Vec<Vec<u8>>,
    row_data: &'a [u8],
    prop_type: &PropertyType,
    page_map_offset: &u16,
    offset: &u16,
    value_size: &u8,
) -> nom::IResult<&'a [u8], Value> {
    let mut value = Value::Null;
    println!("TC offset: {offset}");
    let (value_start, _) = take(*offset)(row_data)?;
    let (_, value_data) = take(*value_size)(value_start)?;
    println!("TC Value data: {value_data:?}");

    match prop_type {
        PropertyType::Int16 => {
            let (_, prop_value) = nom_unsigned_two_bytes(value_data, Endian::Le)?;
            value = serde_json::to_value(&prop_value).unwrap_or_default();
        }
        PropertyType::Int32 => {
            let (_, prop_value) = nom_unsigned_four_bytes(value_data, Endian::Le)?;
            value = serde_json::to_value(prop_value).unwrap_or_default();
        }
        PropertyType::Float32 => todo!(),
        PropertyType::Float64 => todo!(),
        PropertyType::Currency => todo!(),
        PropertyType::FloatTime => todo!(),
        PropertyType::ErrorCode => todo!(),
        PropertyType::Bool => {
            let (_, prop_value) = nom_unsigned_one_byte(value_data, Endian::Le)?;
            let prop_bool = if prop_value != 0 { true } else { false };
            value = serde_json::to_value(&prop_bool).unwrap_or_default();
        }
        PropertyType::Int64 => {
            let (_, prop_value) = nom_unsigned_eight_bytes(value_data, Endian::Le)?;
            value = serde_json::to_value(&prop_value).unwrap_or_default();
        }
        PropertyType::String | PropertyType::MultiString => {
            let (_, offset) = nom_unsigned_four_bytes(value_data, Endian::Le)?;
            if offset == 0 {
                return Ok((row_data, value));
            }
            let (block_index, map_start) = get_map_offset(&offset);
            if let Some(block_data) = all_blocks.get(block_index as usize) {
                let (_, prop_value) =
                    get_property_data(block_data, prop_type, page_map_offset, &map_start, &false)
                        .unwrap();
                value = prop_value;
            }
        }
        PropertyType::String8 => todo!(),
        PropertyType::Time => {
            let (_, prop_value) = nom_unsigned_eight_bytes(value_data, Endian::Le)?;
            let timestamp = filetime_to_unixepoch(&prop_value);
            value = serde_json::to_value(&unixepoch_to_iso(&timestamp)).unwrap_or_default();
        }
        PropertyType::Guid => {
            let (_, offset) = nom_unsigned_four_bytes(value_data, Endian::Le)?;
            if offset == 0 {
                return Ok((row_data, value));
            }
            panic!("a real guid?");
            let string_value = format_guid_le_bytes(value_data);
            value = serde_json::to_value(&string_value).unwrap_or_default();
        }
        PropertyType::ServerId => todo!(),
        PropertyType::Restriction => todo!(),
        PropertyType::Binary => {
            let (_, offset) = nom_unsigned_four_bytes(value_data, Endian::Le)?;
            if offset == 0 {
                return Ok((row_data, value));
            }
            let (block_index, map_start) = get_map_offset(&offset);
            if let Some(block_data) = all_blocks.get(block_index as usize) {
                let (_, prop_value) =
                    get_property_data(block_data, prop_type, page_map_offset, &map_start, &false)
                        .unwrap();
                value = prop_value;
            }
        }
        PropertyType::MultiInt16 => todo!(),
        PropertyType::MultiInt32 => {
            let (input, count) = nom_unsigned_four_bytes(value_data, Endian::Le)?;
            let empty = 0;
            if count != empty {
                panic!("multi-int32: {value_data:?}");
            }
        }
        PropertyType::MultiFloat32 => todo!(),
        PropertyType::MultiFloat64 => todo!(),
        PropertyType::MultiCurrency => todo!(),
        PropertyType::MultiFloatTime => todo!(),
        PropertyType::MultiInt64 => todo!(),
        PropertyType::MultiString8 => todo!(),
        PropertyType::MultiTime => todo!(),
        PropertyType::MultiGuid => todo!(),
        PropertyType::MultiBinary => {
            let (input, count) = nom_unsigned_four_bytes(value_data, Endian::Le)?;
            let empty = 0;
            if count != empty {
                panic!("multi-binary: {value_data:?}");
            }
        }
        PropertyType::Unspecified => todo!(),
        PropertyType::Null => todo!(),
        PropertyType::Object => todo!(),
        PropertyType::RuleAction => todo!(),
        PropertyType::Unknown => {
            value = serde_json::to_value(base64_encode_standard(value_data)).unwrap_or_default();
        }
    };

    Ok((row_data, value))
}

fn get_column_definitions<'a>(
    data: &'a [u8],
    column_count: &u8,
) -> nom::IResult<&'a [u8], Vec<TableRows>> {
    let mut col_data = data;
    let mut count = 0;

    let mut values = Vec::new();

    while &count < column_count {
        let (input, property_type) = nom_unsigned_two_bytes(col_data, Endian::Le)?;
        let (input, id) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, offset) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, size) = nom_unsigned_one_byte(input, Endian::Le)?;
        let (input, index) = nom_unsigned_one_byte(input, Endian::Le)?;

        col_data = input;

        let column = ColumnDescriptor {
            property_type: get_property_type(&property_type),
            property_name: property_id_to_name(&format!(
                "0x{:04x?}_0x{:04x?}",
                &id, &property_type
            )),
            id,
            offset,
            size,
            index,
        };

        let row = TableRows {
            value: Value::Null,
            column,
        };

        values.push(row);
        count += 1;
    }

    Ok((col_data, values))
}

pub(crate) fn get_property_type(prop: &u16) -> PropertyType {
    match prop {
        1 => PropertyType::Null,
        0 => PropertyType::Unspecified,
        13 => PropertyType::Object,
        2 => PropertyType::Int16,
        3 => PropertyType::Int32,
        4 => PropertyType::Float32,
        5 => PropertyType::Float64,
        6 => PropertyType::Currency,
        7 => PropertyType::FloatTime,
        10 => PropertyType::ErrorCode,
        11 => PropertyType::Bool,
        20 => PropertyType::Int64,
        31 => PropertyType::String,
        30 => PropertyType::String8,
        64 => PropertyType::Time,
        72 => PropertyType::Guid,
        251 => PropertyType::ServerId,
        253 => PropertyType::Restriction,
        254 => PropertyType::RuleAction,
        258 => PropertyType::Binary,
        4098 => PropertyType::MultiInt16,
        4099 => PropertyType::MultiInt32,
        4100 => PropertyType::MultiFloat32,
        4101 => PropertyType::MultiFloat64,
        4102 => PropertyType::MultiCurrency,
        4103 => PropertyType::MultiFloatTime,
        4116 => PropertyType::MultiInt64,
        4127 => PropertyType::MultiString,
        4126 => PropertyType::MultiString8,
        4160 => PropertyType::MultiTime,
        4168 => PropertyType::MultiGuid,
        4354 => PropertyType::MultiBinary,
        _ => {
            panic!("[outlook] Unknown property type: {prop}");
            PropertyType::Unknown;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        get_column_definitions, get_property_type, get_row_count, get_row_data, ColumnDescriptor,
        TableRows,
    };
    use crate::artifacts::os::windows::outlook::{
        header::NodeID,
        tables::{
            context::{PropertyType, TableInfo},
            header::HeapNode,
            properties::PropertyName,
        },
    };
    use serde_json::Value;
    use std::collections::BTreeMap;

    #[test]
    fn test_get_row_count() {
        let test = [0, 1, 8, 16];
        let rows = get_row_count(&test);
        assert_eq!(rows, 1);
    }

    #[test]
    fn test_get_row_data() {
        let data = [
            2, 32, 0, 0, 60, 0, 0, 0, 160, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 1, 255, 0, 162, 32, 0, 0, 62, 2, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 255, 0, 82, 0, 111, 0, 111,
            0, 116, 0, 32, 0, 45, 0, 32, 0, 80, 0, 117, 0, 98, 0, 108, 0, 105, 0, 99, 0, 82, 0,
            111, 0, 111, 0, 116, 0, 32, 0, 45, 0, 32, 0, 77, 0, 97, 0, 105, 0, 108, 0, 98, 0, 111,
            0, 120, 0, 6, 0, 0, 0, 12, 0, 20, 0, 162, 0, 178, 0, 56, 1, 82, 1, 110, 1,
        ];

        let info = TableInfo {
            block_data: vec![vec![
                110, 1, 236, 124, 64, 0, 0, 0, 0, 0, 0, 0, 181, 4, 4, 0, 96, 0, 0, 0, 124, 15, 64,
                0, 64, 0, 65, 0, 67, 0, 32, 0, 0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 2, 1, 48, 14, 32, 0,
                4, 8, 20, 0, 51, 14, 36, 0, 8, 9, 2, 1, 52, 14, 44, 0, 4, 10, 3, 0, 56, 14, 48, 0,
                4, 11, 31, 0, 1, 48, 8, 0, 4, 2, 3, 0, 2, 54, 12, 0, 4, 3, 3, 0, 3, 54, 16, 0, 4,
                4, 11, 0, 10, 54, 64, 0, 1, 5, 31, 0, 19, 54, 52, 0, 4, 12, 3, 0, 53, 102, 56, 0,
                4, 13, 3, 0, 54, 102, 60, 0, 4, 14, 3, 0, 56, 102, 20, 0, 4, 6, 3, 0, 242, 103, 0,
                0, 4, 0, 3, 0, 243, 103, 4, 0, 4, 1, 20, 0, 244, 103, 24, 0, 8, 7, 2, 32, 0, 0, 0,
                0, 0, 0, 162, 32, 0, 0, 1, 0, 0, 0, 2, 32, 0, 0, 60, 0, 0, 0, 160, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 255, 0, 162,
                32, 0, 0, 62, 2, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
                0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 255, 0, 82, 0, 111, 0, 111, 0, 116, 0, 32, 0, 45, 0,
                32, 0, 80, 0, 117, 0, 98, 0, 108, 0, 105, 0, 99, 0, 82, 0, 111, 0, 111, 0, 116, 0,
                32, 0, 45, 0, 32, 0, 77, 0, 97, 0, 105, 0, 108, 0, 98, 0, 111, 0, 120, 0, 6, 0, 0,
                0, 12, 0, 20, 0, 162, 0, 178, 0, 56, 1, 82, 1, 110, 1,
            ]],
            block_descriptors: BTreeMap::new(),
            rows: vec![0, 1],
            columns: vec![
                TableRows {
                    value: Value::Null,
                    column: ColumnDescriptor {
                        property_type: PropertyType::String,
                        id: 12289,
                        property_name: vec![PropertyName::PidTagDisplayNameW],
                        offset: 8,
                        size: 4,
                        index: 2,
                    },
                },
                TableRows {
                    value: Value::Null,
                    column: ColumnDescriptor {
                        property_type: PropertyType::Int32,
                        id: 13826,
                        property_name: vec![PropertyName::PidTagContentCount],
                        offset: 12,
                        size: 4,
                        index: 3,
                    },
                },
                TableRows {
                    value: Value::Null,
                    column: ColumnDescriptor {
                        property_type: PropertyType::Int32,
                        id: 13827,
                        property_name: vec![PropertyName::PidTagContentUnreadCount],
                        offset: 16,
                        size: 4,
                        index: 4,
                    },
                },
                TableRows {
                    value: Value::Null,
                    column: ColumnDescriptor {
                        property_type: PropertyType::Bool,
                        id: 13834,
                        property_name: vec![PropertyName::PidTagSubfolders],
                        offset: 64,
                        size: 1,
                        index: 5,
                    },
                },
                TableRows {
                    value: Value::Null,
                    column: ColumnDescriptor {
                        property_type: PropertyType::Int32,
                        id: 26610,
                        property_name: vec![PropertyName::PidTagLtpRowId],
                        offset: 0,
                        size: 4,
                        index: 0,
                    },
                },
                TableRows {
                    value: Value::Null,
                    column: ColumnDescriptor {
                        property_type: PropertyType::Int32,
                        id: 26611,
                        property_name: vec![PropertyName::PidTagLtpRowVer],
                        offset: 4,
                        size: 4,
                        index: 1,
                    },
                },
            ],
            include_cols: Vec::new(),
            row_size: 67,
            map_offset: 366,
            node: HeapNode {
                node: NodeID::HeapNode,
                index: 4,
                block_index: 0,
            },
            total_rows: 2,
        };
        let (_, results) = get_row_data(&data, &info).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(
            results[0][0].value,
            Value::String(String::from("Root - Public"))
        );
    }

    #[test]
    fn test_get_column_definitions() {
        let test = [
            2, 1, 48, 14, 32, 0, 4, 8, 20, 0, 51, 14, 36, 0, 8, 9, 2, 1, 52, 14, 44, 0, 4, 10, 3,
            0, 56, 14, 48, 0, 4, 11, 31, 0, 1, 48, 8, 0, 4, 2, 3, 0, 2, 54, 12, 0, 4, 3, 3, 0, 3,
            54, 16, 0, 4, 4, 11, 0, 10, 54, 64, 0, 1, 5, 31, 0, 19, 54, 52, 0, 4, 12, 3, 0, 53,
            102, 56, 0, 4, 13, 3, 0, 54, 102, 60, 0, 4, 14, 3, 0, 56, 102, 20, 0, 4, 6, 3, 0, 242,
            103, 0, 0, 4, 0, 3, 0, 243, 103, 4, 0, 4, 1, 20, 0, 244, 103, 24, 0, 8, 7, 34, 32, 0,
            0, 0, 0, 0, 0, 66, 32, 0, 0, 1, 0, 0, 0, 34, 32, 0, 0, 11, 0, 0, 0, 160, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 66, 32, 0, 0,
            61, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 1, 255, 0, 73, 0, 80, 0, 77, 0, 95, 0, 83, 0, 85, 0, 66, 0, 84, 0, 82, 0, 69,
            0, 69, 0, 78, 0, 79, 0, 78, 0, 95, 0, 73, 0, 80, 0, 77, 0, 95, 0, 83, 0, 85, 0, 66, 0,
            84, 0, 82, 0, 69, 0, 69, 0, 6, 0, 0, 0, 12, 0, 20, 0, 162, 0, 178, 0, 56, 1, 78, 1,
            108, 1,
        ];
        let (_, rows) = get_column_definitions(&test, &15).unwrap();
        assert_eq!(rows.len(), 15);
    }

    #[test]
    fn test_get_property_type() {
        let test = 13;
        let prop = get_property_type(&test);
        assert_eq!(prop, PropertyType::Object);
    }
}
