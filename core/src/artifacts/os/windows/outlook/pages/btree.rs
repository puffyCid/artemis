use std::{collections::BTreeMap, io::BufReader};

use super::page::{page_type, PageType};
use crate::{
    artifacts::os::windows::outlook::{
        error::OutlookError,
        header::{get_node_ids, FormatType, Node},
    },
    filesystem::ntfs::reader::read_bytes,
    utils::nom_helper::{
        nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_two_bytes, Endian,
    },
};
use log::error;
use nom::{
    bits,
    bytes::complete::take,
    error::ErrorKind,
    number::complete::{le_u32, le_u64},
};
use ntfs::NtfsFile;

#[derive(Debug)]
pub(crate) struct BtreeTable {
    data: Vec<u8>,
    number_entries: u16,
    max_number_entries: u16,
    entry_size: u8,
    node_level: NodeLevel,
    level: u8,
    page_type: PageType,
}

#[derive(PartialEq, Debug)]
enum NodeLevel {
    LeafNode,
    BranchNode,
}

pub(crate) fn get_node_btree<T: std::io::Seek + std::io::Read>(
    ntfs_file: Option<&NtfsFile<'_>>,
    fs: &mut BufReader<T>,
    node_offset: &u64,
    size: &u64,
    format: &FormatType,
    node_tree: &mut Vec<BTreeMap<u32, LeafNodeData>>,
) -> Result<(), OutlookError> {
    let page_result = read_bytes(node_offset, *size, ntfs_file, fs).unwrap();
    let (_, page) = parse_btree_page(&page_result, format).unwrap();

    if page.page_type == PageType::Unknown {
        // We are done
        return Ok(());
    }
    if page.node_level == NodeLevel::BranchNode {
        let (_, branch_nodes) = parse_branch_data(&page.data, format).unwrap();
        for node in branch_nodes {
            println!("branch: {node:?}");
            get_node_btree(ntfs_file, fs, &node.offset, size, format, node_tree)?;
        }
    } else {
        let (_, leaf_node) = parse_leaf_node_data(&page.data, format).unwrap();
        println!("first leaf list: {}", leaf_node.len());
        let mut tree = BTreeMap::new();
        for node in leaf_node {
            if node.node.node_id_num == 0 && node.node.node == 0 {
                println!("skip?: {node:?}");
                continue;
            }
            println!("my node: {node:?}");

            if let Some(value) = tree.get(&node.node.node) {
                println!("The dupe: {value:?}");
            }
            tree.insert(node.node.node, node);
        }
        node_tree.push(tree);
    }

    println!("{}", node_tree.len());

    Ok(())
}

pub(crate) fn get_block_btree<T: std::io::Seek + std::io::Read>(
    ntfs_file: Option<&NtfsFile<'_>>,
    fs: &mut BufReader<T>,
    node_offset: &u64,
    size: &u64,
    format: &FormatType,
    block_tree: &mut Vec<BTreeMap<u64, LeafBlockData>>,
) -> Result<(), OutlookError> {
    let page_result = read_bytes(node_offset, *size, ntfs_file, fs).unwrap();
    let (_, page) = parse_btree_page(&page_result, format).unwrap();

    if page.page_type == PageType::Unknown {
        // We are done
        return Ok(());
    }
    if page.node_level == NodeLevel::BranchNode {
        let (_, branch_nodes) = parse_branch_data(&page.data, format).unwrap();
        for node in branch_nodes {
            println!("branch: {node:?}");
            get_block_btree(ntfs_file, fs, &node.offset, size, format, block_tree)?;
        }
    } else {
        let (_, leaf_block) = parse_leaf_block_data(&page.data, format).unwrap();
        println!("first leaf list: {}", leaf_block.len());
        let mut tree: BTreeMap<u64, LeafBlockData> = BTreeMap::new();
        for block in leaf_block {
            if block.index_id == 0 && block.index == 0 {
                println!("skip?: {block:?}");
                continue;
            }
            println!("my block: {block:?}");
            println!("my block lsb cleared?: {}", (block.index_id & 0xfffffffe));

            if let Some(value) = tree.get(&block.index_id) {
                println!("The dupe: {value:?}");
            }
            tree.insert(block.index_id, block);
        }
        block_tree.push(tree);
    }

    Ok(())
}

pub(crate) fn parse_btree_page<'a>(
    data: &'a [u8],
    format: &FormatType,
) -> nom::IResult<&'a [u8], BtreeTable> {
    let (size, adjust) = match format {
        FormatType::ANSI32 => (512, 16),
        FormatType::Unicode64 => (512, 24),
        FormatType::Unicode64_4k => (4096, 40),
        FormatType::Unknown => {
            // We should never get here
            return Err(nom::Err::Failure(nom::error::Error::new(
                data,
                ErrorKind::Fail,
            )));
        }
    };

    let (input, table_data) = take((size - adjust) as u32)(data)?;
    let (input, number_entries) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, max_number_entries) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, entry_size) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, node_level) = nom_unsigned_one_byte(input, Endian::Le)?;

    let padding_size: u8 = 10;
    let (input, _padding) = take(padding_size)(input)?;
    let (input, page_type) = page_type(input)?;

    // Don't care about the rest of the format

    let btree = BtreeTable {
        data: table_data.to_vec(),
        page_type,
        number_entries,
        max_number_entries,
        entry_size,
        node_level: if node_level == 0 {
            NodeLevel::LeafNode
        } else {
            NodeLevel::BranchNode
        },
        level: node_level,
    };

    Ok((input, btree))
}

#[derive(PartialEq, Debug)]
pub(crate) struct BranchData {
    /**Top Level NodeID's should be unique */
    node: Node,
    back_pointer: u64,
    offset: u64,
}

pub(crate) fn parse_branch_data<'a>(
    data: &'a [u8],
    format: &FormatType,
) -> nom::IResult<&'a [u8], Vec<BranchData>> {
    let mut branch_data = data;
    let mut branch_nodes = Vec::new();

    // Size depends on Outlook file format
    let size: u8 = if format == &FormatType::ANSI32 { 4 } else { 8 };

    while !branch_data.is_empty() && branch_data.len() >= (size * 3) as usize {
        let (input, node_data) = take(size)(branch_data)?;
        let result = get_node_ids(node_data);
        let node = match result {
            Ok((_, value)) => value,
            Err(err) => {
                error!("[outlook] Failed to parse node id data for node branch: {err:?}");
                return Err(nom::Err::Failure(nom::error::Error::new(
                    data,
                    ErrorKind::Fail,
                )));
            }
        };
        if node.node == 0 && node.node_id_num == 0 && node.node_id_value == 0 {
            // We are done
            break;
        }

        let (input, back_data) = take(size)(input)?;
        let (input, file_data) = take(size)(input)?;
        branch_data = input;

        let (back_pointer, offset) = if format == &FormatType::ANSI32 {
            let (_, back_pointer) = le_u32(back_data)?;
            let (_, offset) = le_u32(file_data)?;
            (back_pointer as u64, offset as u64)
        } else {
            let (_, back_pointer) = le_u64(back_data)?;
            let (_, offset) = le_u64(file_data)?;
            (back_pointer, offset)
        };

        let branch = BranchData {
            node,
            back_pointer,
            offset,
        };
        branch_nodes.push(branch);
    }

    Ok((branch_data, branch_nodes))
}

#[derive(PartialEq, Debug)]
pub(crate) struct LeafNodeData {
    node: Node,
    /**Block ID. Points to the main data for this item (Associated Descriptor Items 0x7cec, 0xbcec, or 0x0101) via the index1 tree (https://www.five-ten-sg.com/libpst/rn01re05.html) */
    block_offset_data_id: u64,
    /**Block ID subnode. Is zero or points to an Associated Tree Item 0x0002 via the index1 tree (https://www.five-ten-sg.com/libpst/rn01re05.html) */
    block_offset_descriptor_id: u64,
    /**If node is a child of `Folder Object`. This is the Node ID for the folder */
    parent_node_index: u32,
}

/**
 * Parse Leaf Btree data.
 * Also called "64 bit Index 2 Leaf Node" - https://www.five-ten-sg.com/libpst/rn01re05.html
 */
pub(crate) fn parse_leaf_node_data<'a>(
    data: &'a [u8],
    format: &FormatType,
) -> nom::IResult<&'a [u8], Vec<LeafNodeData>> {
    let mut leaf_data = data;
    let mut leaf_nodes = Vec::new();

    // Size depends on Outlook file format
    let size: u8 = if format == &FormatType::ANSI32 { 4 } else { 8 };
    let min_size: usize = if format == &FormatType::ANSI32 {
        16
    } else {
        32
    };

    while leaf_data.len() >= min_size {
        let (input, node_data) = take(size)(leaf_data)?;
        let result = get_node_ids(node_data);
        let node = match result {
            Ok((_, value)) => value,
            Err(err) => {
                error!("[outlook] Failed to parse node id data for node leaf: {err:?}");
                return Err(nom::Err::Failure(nom::error::Error::new(
                    data,
                    ErrorKind::Fail,
                )));
            }
        };

        let (input, block_index_data) = take(size)(input)?;
        let (input, block_descriptor_data) = take(size)(input)?;

        let (mut input, parent_node_index) = nom_unsigned_four_bytes(input, Endian::Le)?;
        if format != &FormatType::ANSI32 {
            let (remaining, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
            input = remaining;
        }

        leaf_data = input;
        let (block_offset_data_id, block_offset_descriptor_id) = if format == &FormatType::ANSI32 {
            let (_, back_pointer) = le_u32(block_index_data)?;
            let (_, block_offset) = le_u32(block_descriptor_data)?;
            (back_pointer as u64, block_offset as u64)
        } else {
            let (_, back_pointer) = le_u64(block_index_data)?;
            let (_, block_offset) = le_u64(block_descriptor_data)?;
            (back_pointer, block_offset)
        };

        let leaf = LeafNodeData {
            node,
            block_offset_data_id,
            block_offset_descriptor_id,
            parent_node_index,
        };
        leaf_nodes.push(leaf);
    }

    Ok((leaf_data, leaf_nodes))
}

#[derive(PartialEq, Debug)]
pub(crate) struct LeafBlockData {
    /**
     * Need to clear the first LSB when searching?
     * Second LSB used to determine if block is internal or external
     *  - LSB: 0 - external
     *  - LSB:1 - internal (used for array and local descriptors (nodes?))
     * */
    pub(crate) index_id: u64,
    pub(crate) block_type: BlockType,
    pub(crate) index: u64,
    pub(crate) block_offset: u64,
    pub(crate) size: u16,
    pub(crate) reference_count: u16,
    pub(crate) file_offset_allocation_table: u32,
}

#[derive(PartialEq, Debug)]
pub(crate) enum BlockType {
    Internal,
    External,
}

pub(crate) fn parse_leaf_block_data<'a>(
    data: &'a [u8],
    format: &FormatType,
) -> nom::IResult<&'a [u8], Vec<LeafBlockData>> {
    let mut leaf_data = data;
    let mut leaf_blocks = Vec::new();

    // Size depends on Outlook file format
    let size: u8 = if format == &FormatType::ANSI32 { 4 } else { 8 };
    let min_size: usize = if format == &FormatType::ANSI32 {
        12
    } else {
        24
    };

    while leaf_data.len() >= min_size {
        let (input, index_data) = take(size)(leaf_data)?;
        let (_, (block_type, index)) = check_block_bits(index_data).unwrap();

        let (input, block_data) = take(size)(input)?;
        let (input, size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (mut input, reference_count) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let (index_id, block_offset) = if format == &FormatType::ANSI32 {
            let (_, index_id) = le_u32(index_data)?;
            let (_, block_offset) = le_u32(block_data)?;
            (index_id as u64, block_offset as u64)
        } else {
            let (_, index_id) = le_u64(index_data)?;
            let (_, block_offset) = le_u64(block_data)?;
            (index_id, block_offset)
        };

        let mut leaf = LeafBlockData {
            index_id,
            block_offset,
            block_type,
            index,
            size,
            reference_count,
            file_offset_allocation_table: 0,
        };
        if format != &FormatType::ANSI32 {
            let (remaining, allocation_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
            leaf.file_offset_allocation_table = allocation_offset;
            input = remaining;
        }

        leaf_data = input;
        leaf_blocks.push(leaf);
    }

    Ok((leaf_data, leaf_blocks))
}

fn check_block_bits(data: &[u8]) -> nom::IResult<(&[u8], usize), (BlockType, u64)> {
    let bit_size: u8 = 1;
    let ((input, offset), _clear): ((&[u8], usize), u8) =
        bits::complete::take(bit_size)((data, 0))?;
    let ((input, offset), block_type): ((&[u8], usize), u8) =
        bits::complete::take(bit_size)((input, offset))?;

    let block = if block_type == 1 {
        BlockType::Internal
    } else {
        BlockType::External
    };
    let value_size: u8 = 30;
    let ((input, offset), value) = bits::complete::take(value_size)((input, offset))?;
    Ok(((input, offset), (block, value)))
}

#[cfg(test)]
mod tests {
    use super::{get_node_btree, parse_btree_page};
    use crate::{
        artifacts::os::windows::outlook::{
            header::{FormatType, NodeID},
            pages::{
                btree::{
                    get_block_btree, parse_branch_data, parse_leaf_block_data,
                    parse_leaf_node_data, NodeLevel,
                },
                page::PageType,
            },
        },
        filesystem::files::{file_reader, read_file},
    };
    use std::{io::BufReader, path::PathBuf};

    #[test]
    fn test_get_node_btree() {
        let reader =
            file_reader("C:\\Users\\bob\\Desktop\\azur3m3m1crosoft@outlook.com.ost").unwrap();
        let mut buf_reader = BufReader::new(reader);
        let mut tree = Vec::new();
        get_node_btree(
            None,
            &mut buf_reader,
            &18432000,
            &4096,
            &FormatType::Unicode64_4k,
            &mut tree,
        )
        .unwrap();

        assert_eq!(tree.len(), 6);
    }

    #[test]
    fn test_get_block_btree() {
        let reader =
            file_reader("C:\\Users\\bob\\Desktop\\azur3m3m1crosoft@outlook.com.ost").unwrap();
        let mut buf_reader = BufReader::new(reader);
        let mut tree = Vec::new();

        get_block_btree(
            None,
            &mut buf_reader,
            &18800640,
            &4096,
            &FormatType::Unicode64_4k,
            &mut tree,
        )
        .unwrap();
    }

    #[test]
    fn test_parse_btree_node_page() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/node.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, results) = parse_btree_page(&data, &FormatType::Unicode64_4k).unwrap();
        assert_eq!(results.data.len(), 4056);
        assert_eq!(results.page_type, PageType::NodeBtree);
        assert_eq!(results.node_level, NodeLevel::BranchNode);
        assert_eq!(results.level, 1);
        assert_eq!(results.max_number_entries, 169);
        assert_eq!(results.entry_size, 24);
        assert_eq!(results.number_entries, 6);
    }

    #[test]
    fn test_parse_btree_block_page() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/block.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, results) = parse_btree_page(&data, &FormatType::Unicode64_4k).unwrap();
        assert_eq!(results.data.len(), 4056);
        assert_eq!(results.page_type, PageType::BlockBtree);
        assert_eq!(results.node_level, NodeLevel::BranchNode);
        assert_eq!(results.level, 1);
        assert_eq!(results.max_number_entries, 169);
        assert_eq!(results.entry_size, 24);
        assert_eq!(results.number_entries, 31);

        let (_, blocks) = parse_branch_data(&results.data, &FormatType::Unicode64_4k).unwrap();
        println!("{blocks:?}");
    }

    #[test]
    fn test_parse_branch_data() {
        let test = [
            33, 0, 0, 0, 0, 0, 0, 0, 16, 86, 0, 0, 0, 0, 0, 0, 0, 128, 25, 1, 0, 0, 0, 0, 205, 33,
            0, 0, 0, 0, 0, 0, 136, 85, 0, 0, 0, 0, 0, 0, 0, 224, 61, 1, 0, 0, 0, 0, 239, 129, 0, 0,
            0, 0, 0, 0, 37, 86, 0, 0, 0, 0, 0, 0, 0, 48, 25, 1, 0, 0, 0, 0, 38, 0, 8, 0, 0, 0, 0,
            0, 124, 85, 0, 0, 0, 0, 0, 0, 0, 224, 33, 1, 0, 0, 0, 0, 132, 8, 32, 0, 0, 0, 0, 0,
            176, 64, 0, 0, 0, 0, 0, 0, 0, 80, 34, 1, 0, 0, 0, 0, 132, 23, 32, 0, 0, 0, 0, 0, 6, 84,
            0, 0, 0, 0, 0, 0, 0, 128, 56, 1, 0, 0, 0, 0,
        ];
        let (_, nodes) = parse_branch_data(&test, &FormatType::Unicode64_4k).unwrap();
        assert_eq!(nodes.len(), 6);
        assert_eq!(nodes[0].back_pointer, 22032);
        assert_eq!(nodes[0].offset, 18448384);
        assert_eq!(nodes[0].node.node_id, NodeID::Message);
        assert_eq!(nodes[0].node.node_id_value, 16777216);
        println!("{nodes:?}");
    }

    #[test]
    fn test_parse_btree_leaf_node() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/btree_leaf_node.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, results) = parse_btree_page(&data, &FormatType::Unicode64_4k).unwrap();
        assert_eq!(results.data.len(), 4056);
        assert_eq!(results.page_type, PageType::NodeBtree);
        assert_eq!(results.node_level, NodeLevel::LeafNode);
        assert_eq!(results.level, 0);
        assert_eq!(results.max_number_entries, 126);
        assert_eq!(results.entry_size, 32);
        assert_eq!(results.number_entries, 117);

        let (_, leafs) = parse_leaf_node_data(&results.data, &FormatType::Unicode64_4k).unwrap();
        println!("{leafs:?}");
    }

    #[test]
    fn test_parse_btree_leaf_block() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/btree_leaf_block.raw");

        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let (_, results) = parse_btree_page(&data, &FormatType::Unicode64_4k).unwrap();
        assert_eq!(results.data.len(), 4056);
        assert_eq!(results.page_type, PageType::BlockBtree);
        assert_eq!(results.node_level, NodeLevel::LeafNode);
        assert_eq!(results.level, 0);
        assert_eq!(results.max_number_entries, 169);
        assert_eq!(results.entry_size, 24);
        assert_eq!(results.number_entries, 100);

        let (_, leafs) = parse_leaf_block_data(&results.data, &FormatType::Unicode64_4k).unwrap();
        println!("{leafs:?}");
    }

    #[test]
    fn test_parse_leaf_node() {
        let test = [
            33, 0, 0, 0, 0, 0, 0, 0, 188, 16, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0,
        ];
        let (_, nodes) = parse_leaf_node_data(&test, &FormatType::Unicode64_4k).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].node.node_id, NodeID::Message);
        assert_eq!(nodes[0].node.node_id_num, 4);
        assert_eq!(nodes[0].block_offset_data_id, 69820);
        assert_eq!(nodes[0].block_offset_descriptor_id, 0);
        assert_eq!(nodes[0].parent_node_index, 0);
        assert_eq!(nodes[0].node.node_id_value, 16777216);
    }

    #[test]
    fn test_parse_leaf_block() {
        let test = [
            4, 0, 0, 0, 0, 0, 0, 0, 0, 80, 2, 0, 0, 0, 0, 0, 172, 0, 172, 0, 42, 0, 0, 0,
        ];
        let (_, nodes) = parse_leaf_block_data(&test, &FormatType::Unicode64_4k).unwrap();
        assert_eq!(nodes.len(), 1);
        assert_eq!(nodes[0].block_offset, 151552);
        assert_eq!(nodes[0].file_offset_allocation_table, 42);
        assert_eq!(nodes[0].index_id, 4);
        assert_eq!(nodes[0].reference_count, 172);
        assert_eq!(nodes[0].size, 172);
    }
}
