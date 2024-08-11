use crate::{
    artifacts::os::windows::outlook::{
        header::{get_node_ids, FormatType, Node},
        pages::btree::NodeLevel,
    },
    utils::nom_helper::{
        nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
        nom_unsigned_two_bytes, Endian,
    },
};
use nom::bytes::complete::take;
use std::collections::BTreeMap;

#[derive(Debug)]
pub(crate) struct DescriptorData {
    node_level: NodeLevel,
    node: Node,
    /**Only on `NodeLevel::Branch` */
    block_subnode_id: u64,
    /**Only on `NodeLevel::Leaf` */
    block_data_id: u64,
    /**Only on `NodeLevel::Leaf` */
    block_descriptor_id: u64,
}

pub(crate) fn parse_descriptor_block<'a>(
    data: &'a [u8],
    format: &FormatType,
) -> nom::IResult<&'a [u8], BTreeMap<u64, DescriptorData>> {
    let (input, sig) = nom_unsigned_one_byte(data, Endian::Le)?;
    let (input, level) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (mut input, entries) = nom_unsigned_two_bytes(input, Endian::Le)?;

    if format != &FormatType::ANSI32 {
        let (remaining, _padding) = nom_unsigned_four_bytes(input, Endian::Le)?;
        input = remaining;
    }

    let mut count = 0;
    let mut descriptor_tree = BTreeMap::new();
    while count < entries {
        count += 1;
        if format == &FormatType::ANSI32 {
            let (remaining, node_data) = take(size_of::<u32>())(input)?;
            let (_, node) = get_node_ids(node_data)?;
            let mut tree = DescriptorData {
                node_level: if level == 0 {
                    NodeLevel::LeafNode
                } else {
                    NodeLevel::BranchNode
                },
                node,
                block_subnode_id: 0,
                block_data_id: 0,
                block_descriptor_id: 0,
            };
            if tree.node_level == NodeLevel::BranchNode {
                let (remaining, block_subnode_id) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
                tree.block_subnode_id = block_subnode_id as u64;
                input = remaining;

                descriptor_tree.insert(tree.node.node_id_num, tree);
                continue;
            }
            let (remaining, block_data_id) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
            let (remaining, block_descriptor_id) = nom_unsigned_four_bytes(remaining, Endian::Le)?;
            tree.block_data_id = block_data_id as u64;
            tree.block_descriptor_id = block_descriptor_id as u64;
            input = remaining;

            descriptor_tree.insert(tree.node.node_id_num, tree);
            continue;
        }

        let (remaining, node_data) = take(size_of::<u64>())(input)?;
        let (_, node) = get_node_ids(node_data)?;
        let mut tree = DescriptorData {
            node_level: if level == 0 {
                NodeLevel::LeafNode
            } else {
                NodeLevel::BranchNode
            },
            node,
            block_subnode_id: 0,
            block_data_id: 0,
            block_descriptor_id: 0,
        };
        if tree.node_level == NodeLevel::BranchNode {
            let (remaining, block_subnode_id) = nom_unsigned_eight_bytes(remaining, Endian::Le)?;
            tree.block_subnode_id = block_subnode_id;
            input = remaining;

            descriptor_tree.insert(tree.node.node_id_num, tree);
            continue;
        }
        let (remaining, block_data_id) = nom_unsigned_eight_bytes(remaining, Endian::Le)?;
        let (remaining, block_descriptor_id) = nom_unsigned_eight_bytes(remaining, Endian::Le)?;
        tree.block_data_id = block_data_id;
        tree.block_descriptor_id = block_descriptor_id;
        input = remaining;

        descriptor_tree.insert(tree.node.node_id_num, tree);
    }

    Ok((input, descriptor_tree))
}

#[cfg(test)]
mod tests {
    use super::parse_descriptor_block;
    use crate::artifacts::os::windows::outlook::header::FormatType;

    #[test]
    fn test_parse_descriptor_block() {
        let test = [
            2, 0, 2, 0, 0, 0, 0, 0, 33, 5, 0, 0, 0, 0, 0, 0, 16, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 65, 5, 0, 0, 0, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 56, 0,
            20, 84, 201, 62, 214, 166, 22, 0, 0, 0, 0, 0, 0, 0, 2, 0, 56, 0, 0, 0, 0, 0,
        ];
        let (_, results) = parse_descriptor_block(&test, &FormatType::Unicode64_4k).unwrap();
        assert_eq!(results.len(), 2);
    }
}
