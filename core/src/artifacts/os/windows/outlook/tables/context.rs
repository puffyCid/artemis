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
    MultiIn16,
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
    fn parse_table_context<'a>(
        &mut self,
        data: &'a [u8],
        descriptors: &BTreeMap<u64, DescriptorData>,
    ) -> nom::IResult<&'a [u8], TableContext>;

    fn get_descriptor_data(&mut self, descriptor: &DescriptorData)
        -> Result<Vec<u8>, OutlookError>;
}

impl<T: std::io::Seek + std::io::Read> OutlookTableContext<T> for OutlookReader<T> {
    fn parse_table_context<'a>(
        &mut self,
        data: &'a [u8],
        descriptors: &BTreeMap<u64, DescriptorData>,
    ) -> nom::IResult<&'a [u8], TableContext> {
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

        let mut descriptor_data = Vec::new();
        if row.node == NodeID::LocalDescriptors {
            println!("TC Desc: {descriptors:?}");
            if let Some(descriptor) = descriptors.get(&(row.index as u64)) {
                descriptor_data = self.get_descriptor_data(descriptor).unwrap();
            }
        }

        let (input, _padding) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let mut table = TableContext {
            sig,
            number_column_definitions,
            array_end_32bit,
            array_end_16bit,
            array_end_8bit,
            array_end_offset,
            row_index,
            row,
            rows: Vec::new(),
        };
        let row_count = get_row_count(&header.page_map.allocation_table);

        let (mut input, mut rows) =
            get_column_definitions(input, &table.number_column_definitions, &row_count)?;
        println!("Rows: {}", rows.len());

        let mut count = 0;

        while count < row_count {
            let (remaining, row_id) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (remaining, index) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
            input = remaining;
            count += 1;

            println!("Row ID: {row_id} - Index: {index}");
        }

        // If we have descriptor data then part of the Table row is stored in the descriptor
        if !descriptor_data.is_empty() {
            let result = get_row_data(
                &descriptor_data,
                &mut rows,
                table.array_end_offset,
                &table.array_end_8bit,
                &header.page_map_offset,
                data,
            );

            table.rows = rows;
            return Ok((input, table));
        }

        let (input, _) = get_row_data(
            input,
            &mut rows,
            table.array_end_offset,
            &table.array_end_8bit,
            &header.page_map_offset,
            data,
        )?;

        table.rows = rows;

        Ok((input, table))
    }

    fn get_descriptor_data(
        &mut self,
        descriptor: &DescriptorData,
    ) -> Result<Vec<u8>, OutlookError> {
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

fn get_row_count(map: &[u16]) -> u16 {
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
    count
}

fn get_row_data<'a>(
    data: &'a [u8],
    rows: &mut [Vec<TableRows>],
    row_data_size: u16,
    cell_map_start: &u16,
    page_map_offset: &u16,
    original_data: &'a [u8],
) -> nom::IResult<&'a [u8], ()> {
    let mut input = data;
    for row in rows {
        //let (cell_map, _) = take(*cell_map_start)(input)?;
        println!("Row size: {row_data_size}");
        let (reamining, row_data) = take(row_data_size)(input)?;
        println!("row data: {row_data:?}");

        for column in row {
            /*
            if let Some(cell) = cell_map.get((column.column.index / 8) as usize) {
               let skip = cell & (1 << (7 - column.column.index % 8));
               let cell_not_exists = 0;
               if skip == cell_not_exists {
                continue;
               }
            }*/
            let (col_data_start, _) = take(column.column.offset)(row_data)?;
            println!("column: {column:?}");
            let (_, value) = parse_row_data(
                original_data,
                row_data,
                &column.column.property_type,
                page_map_offset,
                &(column.column.offset as u32),
                &column.column.size,
            )?;
            println!("col value: {value:?}");
            column.value = value;
        }
        input = reamining;
    }
    Ok((data, ()))
}

fn parse_row_data<'a>(
    original_data: &'a [u8],
    row_data: &'a [u8],
    prop_type: &PropertyType,
    page_map_offset: &u16,
    offset: &u32,
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
            println!("string offset: {offset}");
            println!("heap?:{:?}", get_heap_node_id(&offset));
            if offset != 0 && page_map_offset != &0 {
                let (_, prop_value) =
                    get_property_data(original_data, prop_type, page_map_offset, &offset, &false)?;
                value = prop_value;
                //panic!("wrong?: {value:?}");
            }
        }
        PropertyType::String8 => todo!(),
        PropertyType::Time => {
            let (_, prop_value) = nom_unsigned_eight_bytes(value_data, Endian::Le)?;
            let timestamp = filetime_to_unixepoch(&prop_value);
            value = serde_json::to_value(&unixepoch_to_iso(&timestamp)).unwrap_or_default();
        }
        PropertyType::Guid => {
            let string_value = format_guid_le_bytes(value_data);
            value = serde_json::to_value(&string_value).unwrap_or_default();
        }
        PropertyType::ServerId => todo!(),
        PropertyType::Restriction => todo!(),
        PropertyType::Binary => {
            let (_, offset) = nom_unsigned_four_bytes(value_data, Endian::Le)?;
            let empty = 0;
            if offset != empty {
                println!("binary offset: {offset}");
                let (_, prop_value) =
                    get_property_data(original_data, prop_type, page_map_offset, &offset, &false)?;
                value = prop_value;
            }
        }
        PropertyType::MultiIn16 => todo!(),
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
    rows: &u16,
) -> nom::IResult<&'a [u8], Vec<Vec<TableRows>>> {
    let mut col_data = data;
    let mut count = 0;

    let mut row_values = Vec::new();
    let mut row_count = 0;
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

    // Each row has same number of column descriptors
    while &row_count < rows {
        row_values.push(values.clone());
        row_count += 1;
    }

    Ok((col_data, row_values))
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
        4098 => PropertyType::MultiIn16,
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
    use crate::artifacts::os::windows::outlook::tables::{
        context::PropertyType, properties::PropertyName,
    };
    use serde_json::Value;

    #[test]
    fn test_get_row_count() {
        let test = [0, 1, 8, 16];
        let rows = get_row_count(&test);
        assert_eq!(rows, 1);
    }

    #[test]
    fn test_get_row_data() {
        let data = [
            34, 32, 0, 0, 11, 0, 0, 0, 160, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 66, 32, 0, 0, 61, 0, 0, 0, 192, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 255, 0, 73, 0, 80, 0, 77,
            0, 95, 0, 83, 0, 85, 0, 66, 0, 84, 0, 82, 0, 69, 0, 69, 0, 78, 0, 79, 0, 78, 0, 95, 0,
            73, 0, 80, 0, 77, 0, 95, 0, 83, 0, 85, 0, 66, 0, 84, 0, 82, 0, 69, 0, 69, 0, 6, 0, 0,
            0, 12, 0, 20, 0, 162, 0, 178, 0, 56, 1, 78, 1, 108, 1,
        ];
        let test = [
            108, 1, 236, 124, 64, 0, 0, 0, 0, 0, 0, 0, 181, 4, 4, 0, 96, 0, 0, 0, 124, 15, 64, 0,
            64, 0, 65, 0, 67, 0, 32, 0, 0, 0, 128, 0, 0, 0, 0, 0, 0, 0, 2, 1, 48, 14, 32, 0, 4, 8,
            20, 0, 51, 14, 36, 0, 8, 9, 2, 1, 52, 14, 44, 0, 4, 10, 3, 0, 56, 14, 48, 0, 4, 11, 31,
            0, 1, 48, 8, 0, 4, 2, 3, 0, 2, 54, 12, 0, 4, 3, 3, 0, 3, 54, 16, 0, 4, 4, 11, 0, 10,
            54, 64, 0, 1, 5, 31, 0, 19, 54, 52, 0, 4, 12, 3, 0, 53, 102, 56, 0, 4, 13, 3, 0, 54,
            102, 60, 0, 4, 14, 3, 0, 56, 102, 20, 0, 4, 6, 3, 0, 242, 103, 0, 0, 4, 0, 3, 0, 243,
            103, 4, 0, 4, 1, 20, 0, 244, 103, 24, 0, 8, 7, 34, 32, 0, 0, 0, 0, 0, 0, 66, 32, 0, 0,
            1, 0, 0, 0, 34, 32, 0, 0, 11, 0, 0, 0, 160, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 255, 0, 66, 32, 0, 0, 61, 0, 0, 0, 192, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 255, 0, 73, 0,
            80, 0, 77, 0, 95, 0, 83, 0, 85, 0, 66, 0, 84, 0, 82, 0, 69, 0, 69, 0, 78, 0, 79, 0, 78,
            0, 95, 0, 73, 0, 80, 0, 77, 0, 95, 0, 83, 0, 85, 0, 66, 0, 84, 0, 82, 0, 69, 0, 69, 0,
            6, 0, 0, 0, 12, 0, 20, 0, 162, 0, 178, 0, 56, 1, 78, 1, 108, 1,
        ];
        let mut rows = vec![vec![TableRows {
            value: Value::Null,
            column: ColumnDescriptor {
                property_type: PropertyType::Binary,
                id: 3632,
                property_name: vec![PropertyName::Unknown],
                offset: 32,
                size: 4,
                index: 8,
            },
        }]];
        get_row_data(&data, &mut rows, 67, &65, &364, &test).unwrap();
        assert_eq!(rows[0][0].value, Value::Null);
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
        let (_, rows) = get_column_definitions(&test, &15, &2).unwrap();
        assert_eq!(rows.len(), 2);
    }

    #[test]
    fn test_get_property_type() {
        let test = 13;
        let prop = get_property_type(&test);
        assert_eq!(prop, PropertyType::Object);
    }
}
