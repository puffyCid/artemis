use nom::bytes::complete::take;
use serde_json::Value;

use crate::{
    artifacts::os::windows::outlook::tables::header::get_heap_node_id,
    utils::nom_helper::{
        nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
    },
};

use super::{
    header::{HeapNode, HeapPageMap},
    properties::{property_id_to_name, PropertyName},
    property::get_property_data,
};

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
}

#[derive(Debug)]
pub(crate) struct TableRows {
    value: Value,
    column: ColumnDescriptor,
}

#[derive(Debug)]
pub(crate) struct ColumnDescriptor {
    property_type: PropertyType,
    id: u16,
    property_name: Vec<PropertyName>,
    offset: u16,
    size: u8,
    index: u8,
}

#[derive(Debug, PartialEq)]
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

pub(crate) fn parse_table_context<'a>(
    data: &'a [u8],
    map: &[u16],
) -> nom::IResult<&'a [u8], TableContext> {
    let (input, sig) = nom_unsigned_one_byte(data, Endian::Le)?;
    let (input, number_column_definitions) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, array_end_32bit) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, array_end_16bit) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, array_end_8bit) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, array_end_offset) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, table_context_index_reference) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let row_index = get_heap_node_id(&table_context_index_reference);
    println!("{row_index:?}");
    let (input, values_array_index_reference) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let row = get_heap_node_id(&values_array_index_reference);
    println!("{row:?}");

    let (input, _padding) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let table = TableContext {
        sig,
        number_column_definitions,
        array_end_32bit,
        array_end_16bit,
        array_end_8bit,
        array_end_offset,
        row_index,
        row,
    };
    let row_count = get_row_count(map);

    let (mut input, mut rows) =
        get_column_definitions(input, &table.number_column_definitions, &row_count)?;
    println!("{rows:?}");

    let mut count = 0;

    while count < row_count {
        let (remaining, row_id) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (remaining, index) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
        input = remaining;
        count += 1;

        println!("Row ID: {row_id} - Index: {index}");
    }

    let (input, _) = get_row_data(input, &mut rows, table.array_end_offset)?;

    Ok((input, table))
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
) -> nom::IResult<&'a [u8], ()> {
    let mut input = data;
    for row in rows {
        let (reamining, row_data) = take(row_data_size)(input)?;
        for column in row {
            let (_, value) = get_property_data(
                row_data,
                column.column.size as u16,
                &column.column.property_type,
                column.column.offset,
            )?;
            println!("{value:?}");
        }
        input = reamining;
    }
    Ok((data, ()))
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
    while &row_count < rows {
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
        row_count += 1;
        row_values.push(values);
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
    use super::parse_table_context;
    use crate::artifacts::os::windows::outlook::tables::{
        header::table_header, heap_btree::parse_btree_heap,
    };

    #[test]
    fn test_parse_table_context_empty() {
        let test = [
            124, 15, 64, 0, 64, 0, 65, 0, 67, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 2, 1, 48, 14,
            32, 0, 4, 8, 20, 0, 51, 14, 36, 0, 8, 9, 2, 1, 52, 14, 44, 0, 4, 10, 3, 0, 56, 14, 48,
            0, 4, 11, 31, 0, 1, 48, 8, 0, 4, 2, 3, 0, 2, 54, 12, 0, 4, 3, 3, 0, 3, 54, 16, 0, 4, 4,
            11, 0, 10, 54, 64, 0, 1, 5, 31, 0, 19, 54, 52, 0, 4, 12, 3, 0, 53, 102, 56, 0, 4, 13,
            3, 0, 54, 102, 60, 0, 4, 14, 3, 0, 56, 102, 20, 0, 4, 6, 3, 0, 242, 103, 0, 0, 4, 0, 3,
            0, 243, 103, 4, 0, 4, 1, 20, 0, 244, 103, 24, 0, 8, 7, 2, 0, 0, 0, 12, 0, 20, 0, 162,
            0,
        ];

        let (_, table) = parse_table_context(&test, &[]).unwrap();
        println!("{table:?}");
        assert_eq!(table.sig, 124);
        assert_eq!(table.number_column_definitions, 15);
        assert_eq!(table.array_end_32bit, 64);
        assert_eq!(table.array_end_16bit, 64);
        assert_eq!(table.array_end_8bit, 65);
        assert_eq!(table.array_end_offset, 67);
        assert_eq!(table.row_index.index, 1);
        assert_eq!(table.row.index, 0);
    }

    #[test]
    fn test_parse_table_context_ipm() {
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
        let (input, header) = table_header(&test).unwrap();
        println!("{header:?}");

        let (input, heap) = parse_btree_heap(&input).unwrap();
        println!("{heap:?}");
        let (input, table) = parse_table_context(input, &header.page_map.allocation_table).unwrap();
        println!("{table:?}");

        assert_eq!(table.row_index.index, 1);
        assert_eq!(table.row.index, 4);
    }
}
