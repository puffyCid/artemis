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

#[derive(Debug, PartialEq)]
pub(crate) struct DescriptorData {
    pub(crate) node_level: NodeLevel,
    pub(crate) node: Node,
    /**Only on `NodeLevel::Branch` */
    pub(crate) block_subnode_id: u64,
    /**Only on `NodeLevel::Leaf` */
    pub(crate) block_data_id: u64,
    /**Only on `NodeLevel::Leaf` */
    pub(crate) block_descriptor_id: u64,
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
    use crate::artifacts::os::windows::outlook::header::Node;
    use crate::artifacts::os::windows::outlook::header::NodeID::{InternalNode, LocalDescriptors};
    use crate::artifacts::os::windows::outlook::pages::btree::NodeLevel::LeafNode;
    use crate::artifacts::os::windows::outlook::{
        blocks::descriptors::DescriptorData, header::FormatType,
    };

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

    #[test]
    fn test_parse_descriptor_block_root_folder() {
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
        let (_, result) = parse_descriptor_block(&test, &FormatType::Unicode64_4k).unwrap();
        println!("{result:?}");

        assert_eq!(result.len(), 2);
        assert_eq!(
            result.get(&41).unwrap(),
            &DescriptorData {
                node_level: LeafNode,
                node: Node {
                    node_id: InternalNode,
                    node_id_num: 41,
                    node: 1313
                },
                block_subnode_id: 0,
                block_data_id: 16,
                block_descriptor_id: 0
            }
        )
    }

    #[test]
    fn test_parse_descriptor_block_node_id_map() {
        let test = [
            2, 0, 2, 0, 0, 0, 0, 0, 63, 131, 0, 0, 0, 0, 0, 0, 48, 8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 159, 131, 0, 0, 0, 0, 0, 0, 44, 8, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
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
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 56, 0,
            32, 169, 90, 37, 126, 125, 58, 8, 1, 0, 0, 0, 0, 0, 2, 0, 56, 0, 0, 0, 0, 0,
        ];
        let (_, result) = parse_descriptor_block(&test, &FormatType::Unicode64_4k).unwrap();
        println!("{result:?}");

        assert_eq!(result.len(), 2);
        assert_eq!(
            result.get(&1049).unwrap(),
            &DescriptorData {
                node_level: LeafNode,
                node: Node {
                    node_id: LocalDescriptors,
                    node_id_num: 1049,
                    node: 33599
                },
                block_subnode_id: 0,
                block_data_id: 67632,
                block_descriptor_id: 0
            }
        )
    }
}
