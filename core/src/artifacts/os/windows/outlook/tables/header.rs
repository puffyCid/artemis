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
    page_map_offset: u16,
    sig: u8,
    table_type: TableType,
    // value_reference: u32,
    heap_node: HeapNode,
    fill: Vec<FillLevel>,
    page_map: HeapPageMap,
}

#[derive(PartialEq, Debug)]
enum TableType {
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
enum FillLevel {
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
    let (page_data, _) = take(page_map_offset)(data)?;
    let (_, page_map) = heap_page_map(page_data)?;

    let (input, sig) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, table_type) = nom_unsigned_one_byte(input, Endian::Le)?;

    let (input, root_heap) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let heap_node = get_heap_node_id(&root_heap);
    println!("{heap_node:?}");
    let (input, level_data) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let table = TableHeader {
        page_map_offset,
        sig,
        table_type: get_table_type(&table_type),
        // value_reference,
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
    allocation_count: u16,
    free: u16,
    allocation_table: Vec<u16>,
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
}
