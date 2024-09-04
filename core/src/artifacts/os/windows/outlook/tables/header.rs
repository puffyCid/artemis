use crate::{
    artifacts::os::windows::outlook::header::NodeID,
    utils::nom_helper::{
        nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
    },
};
use log::warn;
use nom::bytes::complete::take;

#[derive(Debug)]
pub(crate) struct TableHeader {
    pub(crate) page_map_offset: u16,
    pub(crate) sig: u8,
    pub(crate) table_type: TableType,
    pub(crate) heap_node: HeapNode,
    pub(crate) fill: Vec<FillLevel>,
    pub(crate) page_map: HeapPageMap,
}

#[derive(PartialEq, Debug)]
pub(crate) enum TableType {
    SixC,
    TableContext,
    EightC,
    NineC,
    A5,
    Ac,
    BtreeHeap,
    PropertyContext,
    Cc,
    Unknown,
}

#[derive(PartialEq, Debug)]
pub(crate) enum FillLevel {
    Empty,
    Level1,
    Level2,
    Level3,
    Level4,
    Level5,
    Level6,
    Level7,
    Level8,
    Level9,
    Level10,
    Level11,
    Level12,
    Level13,
    Level14,
    LevelFull,
}

pub(crate) fn table_header(data: &[u8]) -> nom::IResult<&[u8], TableHeader> {
    let (input, page_map_offset) = nom_unsigned_two_bytes(data, Endian::Le)?;

    let (input, sig) = nom_unsigned_one_byte(input, Endian::Le)?;
    let sig_value = 236;
    if sig != sig_value {
        let not_header = TableHeader {
            page_map_offset: 0,
            sig: 0,
            table_type: TableType::Unknown,
            heap_node: HeapNode {
                node: NodeID::Unknown,
                index: 0,
                block_index: 0,
            },
            fill: Vec::new(),
            page_map: HeapPageMap {
                allocation_count: 0,
                free: 0,
                allocation_table: Vec::new(),
            },
        };

        return Ok((input, not_header));
    }
    let (page_data, _) = take(page_map_offset)(data)?;
    let (_, page_map) = heap_page_map(page_data)?;
    let (input, table_type) = nom_unsigned_one_byte(input, Endian::Le)?;

    let (input, root_heap) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let heap_node = get_heap_node_id(&root_heap);
    println!("{heap_node:?}");
    let (input, level_data) = nom_unsigned_four_bytes(input, Endian::Le)?;
    println!("level: {level_data}");

    let table = TableHeader {
        page_map_offset,
        sig,
        table_type: get_table_type(&table_type),
        heap_node,
        fill: Vec::new(),
        page_map,
    };

    Ok((input, table))
}

fn get_table_type(table: &u8) -> TableType {
    match table {
        0x6c => TableType::SixC,
        0x7c => TableType::TableContext,
        0x8c => TableType::EightC,
        0x9c => TableType::NineC,
        0xa5 => TableType::A5,
        0xac => TableType::Ac,
        0xb5 => TableType::BtreeHeap,
        0xbc => TableType::PropertyContext,
        0xcc => TableType::Cc,
        _ => {
            warn!("[outlook] Unknown table type: {table}");
            TableType::Unknown
        }
    }
}

#[derive(Debug)]
pub(crate) struct HeapNode {
    pub(crate) node: NodeID,
    pub(crate) index: u32,
    pub(crate) block_index: u32,
}

pub(crate) fn get_heap_node_id(value: &u32) -> HeapNode {
    let id = match value & 0x1f {
        0x0 => NodeID::HeapNode,
        0x1 => NodeID::InternalNode,
        0x2 => NodeID::NormalFolder,
        0x3 => NodeID::SearchFolder,
        0x4 => NodeID::Message,
        0x5 => NodeID::Attachment,
        0x6 => NodeID::SearchUpdateQueue,
        0x7 => NodeID::SearchCriteria,
        0x8 => NodeID::FolderAssociatedInfo,
        0xa => NodeID::ContentsTableIndex,
        0xb => NodeID::Inbox,
        0xc => NodeID::Outbox,
        0xd => NodeID::HierarchyTable,
        0xe => NodeID::ContentsTable,
        0xf => NodeID::FaiContentsTable,
        0x10 => NodeID::SearchContentsTable,
        0x11 => NodeID::AttachmentTable,
        0x12 => NodeID::RecipientTable,
        0x13 => NodeID::SearchTableIndex,
        0x1f => NodeID::LocalDescriptors,
        _ => {
            warn!("[outlook] Unknown NodeID for Heap BTree: {value}");
            NodeID::Unknown
        }
    };

    let index = (value >> 5) & 0x07ffffff;

    // Will only work for OST files (Outlook 2013+)
    let adjust_index = 19;
    let adjust = 0xffff;
    let block_index = adjust & (value >> adjust_index);
    let node = HeapNode {
        node: id,
        index,
        block_index,
    };

    node
}

#[derive(Debug)]
pub(crate) struct HeapPageMap {
    pub(crate) allocation_count: u16,
    pub(crate) free: u16,
    pub(crate) allocation_table: Vec<u16>,
}

pub(crate) fn heap_page_map(data: &[u8]) -> nom::IResult<&[u8], HeapPageMap> {
    let (input, allocation_count) = nom_unsigned_two_bytes(data, Endian::Le)?;
    let (mut input, free) = nom_unsigned_two_bytes(input, Endian::Le)?;

    let mut count = 0;

    let mut page = HeapPageMap {
        allocation_count,
        free,
        allocation_table: Vec::new(),
    };

    // Need to always add 1 to the count
    while count < allocation_count + 1 {
        let (remaining, value) = nom_unsigned_two_bytes(input, Endian::Le)?;
        input = remaining;

        page.allocation_table.push(value);
        count += 1;
    }

    Ok((input, page))
}

#[cfg(test)]
mod tests {
    use super::table_header;
    use crate::artifacts::os::windows::outlook::{header::NodeID, tables::header::TableType};

    #[test]
    fn test_table_header() {
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
        let (_, header) = table_header(&test).unwrap();
        assert_eq!(header.page_map_offset, 364);
        assert_eq!(header.sig, 236);
        assert_eq!(header.table_type, TableType::TableContext);
        // assert_eq!(header.value_reference, 64);
        assert_eq!(header.heap_node.node, NodeID::HeapNode);
        assert_eq!(header.heap_node.index, 2);
        assert_eq!(header.fill, Vec::new());
        assert_eq!(header.page_map.allocation_count, 6);
        println!("{header:?}");
    }

    #[test]
    fn test_table_header_root() {
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
        let (_, header) = table_header(&test).unwrap();
        assert_eq!(header.table_type, TableType::PropertyContext);
        assert_eq!(header.page_map.allocation_count, 5);
        assert_eq!(
            header.page_map.allocation_table,
            vec![12, 20, 116, 124, 132, 581]
        );

        println!("{header:?}");
    }
}
