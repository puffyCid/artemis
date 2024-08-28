use super::header::{get_heap_node_id, HeapNode};
use crate::{
    artifacts::os::windows::outlook::pages::btree::NodeLevel,
    utils::nom_helper::{nom_unsigned_four_bytes, nom_unsigned_one_byte, Endian},
};

#[derive(Debug)]
pub(crate) struct HeapBtree {
    sig: u8,
    record_entry_size: u8,
    record_value_size: u8,
    pub(crate) level: NodeLevel,
    node: HeapNode,
}

pub(crate) fn parse_btree_heap(data: &[u8]) -> nom::IResult<&[u8], HeapBtree> {
    let (input, sig) = nom_unsigned_one_byte(data, Endian::Le)?;
    let (input, record_entry_size) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, record_value_size) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, level) = nom_unsigned_one_byte(input, Endian::Le)?;
    let (input, node_value) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let node = get_heap_node_id(&node_value);
    println!("{node:?}");

    let table = HeapBtree {
        sig,
        record_entry_size,
        record_value_size,
        level: if level == 0 {
            NodeLevel::LeafNode
        } else {
            NodeLevel::BranchNode
        },
        node,
    };
    Ok((input, table))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::outlook::pages::btree::NodeLevel;

    use super::parse_btree_heap;

    #[test]
    fn test_parse_btree_heap() {
        let test = [181, 4, 4, 0, 96, 0, 0, 0];
        let (_, result) = parse_btree_heap(&test).unwrap();
        assert_eq!(result.sig, 181);
        assert_eq!(result.record_value_size, 4);
        assert_eq!(result.record_entry_size, 4);
        assert_eq!(result.level, NodeLevel::LeafNode);
        assert_eq!(result.node.index, 3);
    }

    #[test]
    fn test_parse_btree_heap_root() {
        let test = [181, 2, 6, 0, 64, 0, 0, 0];
        let (_, result) = parse_btree_heap(&test).unwrap();
        assert_eq!(result.sig, 181);
        assert_eq!(result.record_value_size, 6);
        assert_eq!(result.record_entry_size, 2);
        assert_eq!(result.level, NodeLevel::LeafNode);
        assert_eq!(result.node.index, 2);
    }
}
