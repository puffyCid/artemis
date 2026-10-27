use crate::{
    accessor::{
        error::AccessorResult,
        filesystem::ext4::format::{
            checksum::crc32c_check,
            superblock::{SuperBlock, ext4_error},
        },
    },
    utils::nom_helper::{nom_u16, nom_u32},
};

#[derive(Debug, Clone, Copy)]
pub(crate) struct ExtentLimits {
    pub(crate) block_count: u64,
    pub(crate) capacity: usize,
    pub(crate) checksum_tail: bool,
    pub(crate) expected_depth: Option<u16>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ExtentHeader {
    pub(crate) entries: u16,
    pub(crate) max_entries: u16,
    pub(crate) depth: u16,
    pub(crate) generation: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Extent {
    pub(crate) logical_block: u32,
    pub(crate) length: u16,
    pub(crate) physical_block: u64,
    pub(crate) unwritten: bool,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ExtentIndex {
    pub(crate) logical_block: u32,
    pub(crate) physical_block: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum ExtentNode {
    Leaf {
        header: ExtentHeader,
        extents: Vec<Extent>,
    },
    Index {
        header: ExtentHeader,
        indexes: Vec<ExtentIndex>,
    },
}

impl ExtentNode {
    pub(crate) fn header(&self) -> &ExtentHeader {
        match self {
            Self::Leaf { header, .. } | Self::Index { header, .. } => header,
        }
    }
}

pub(crate) fn parse_extent_node(data: &[u8], limits: &ExtentLimits) -> AccessorResult<ExtentNode> {
    let header_size = 12;
    if data.len() < header_size || limits.capacity < header_size {
        return Err(ext4_error(format!(
            "extent header needs {header_size} bytes, found {} in a {} byte node",
            data.len(),
            limits.capacity
        )));
    }

    let header = parse_header(data)?;
    if header.max_entries == 0 {
        return Err(ext4_error("extent max entries is 0"));
    }

    if header.entries > header.max_entries {
        return Err(ext4_error(format!(
            "extent entries {} exceed max {}",
            header.entries, header.max_entries
        )));
    }

    // Attempt to prevent corrupted extent depth value
    let max_depth = 32;
    if header.depth > max_depth {
        return Err(ext4_error(format!(
            "extent depth {} exceeds {max_depth}",
            header.depth
        )));
    }

    if let Some(expected) = limits.expected_depth
        && header.depth != expected
    {
        return Err(ext4_error(format!(
            "extent depth {} does not match expected {expected}",
            header.depth
        )));
    }

    let entry_size = 12;
    let slots_end = header_size + usize::from(header.max_entries) * entry_size;
    if slots_end > limits.capacity || data.len() < slots_end {
        return Err(ext4_error(format!(
            "extent node needs {slots_end} bytes, found {} in a {} byte node",
            data.len(),
            limits.capacity
        )));
    }

    let tail_size = 4;
    if limits.checksum_tail {
        let tail_end = slots_end + tail_size;
        if tail_end > limits.capacity || data.len() < tail_end {
            return Err(ext4_error("extent checksum is outside the node"));
        }
    }

    let entries_end = header_size + header.entries as usize * entry_size;

    if header.depth == 0 {
        let extents = parse_leaves(&data[header_size..entries_end], limits.block_count)?;
        Ok(ExtentNode::Leaf { header, extents })
    } else {
        let indexes = parse_indexes(&data[header_size..entries_end], limits.block_count)?;
        Ok(ExtentNode::Index { header, indexes })
    }
}

pub(crate) fn verify_extent_block(
    superblock: &SuperBlock,
    checksum_seed: Option<u32>,
    inode: u32,
    inode_generation: u32,
    data: &[u8],
    header: &ExtentHeader,
) -> AccessorResult<()> {
    if !superblock.has_read_only(SuperBlock::METADATA_CSUM) {
        return Ok(());
    }

    let seed = checksum_seed.ok_or_else(|| ext4_error("metadata checksum seed is missing"))?;
    let header_size = 12;
    let entry_size = 12;
    let tail_size = 4;

    let tail = header_size + usize::from(header.max_entries) * entry_size;
    if data.len() < tail + tail_size {
        return Err(ext4_error(format!(
            "inode {inode} extent checksum is outside the node"
        )));
    }

    let actual = extent_crc32c(seed, inode, inode_generation, data, header.max_entries);
    let (_, stored) = nom_u32(
        &data[tail..tail + tail_size],
        "truncated extent CRC32C checksum",
    )?;
    if actual != stored {
        return Err(ext4_error(format!(
            "inode {inode} extent checksum mismatch: found {actual:#x}, stored {stored:#x}"
        )));
    }

    Ok(())
}

pub(crate) fn extent_crc32c(
    seed: u32,
    inode: u32,
    inode_generation: u32,
    data: &[u8],
    max_entries: u16,
) -> u32 {
    let mut crc = crc32c_check(seed, &inode.to_le_bytes());
    crc = crc32c_check(crc, &inode_generation.to_le_bytes());

    let header_size = 12;
    let entry_size = 12;
    let covered = header_size + max_entries as usize * entry_size;

    crc32c_check(crc, &data[..covered])
}

fn parse_header(data: &[u8]) -> AccessorResult<ExtentHeader> {
    let (input, magic) = nom_u16(data, "extent magic is truncated")?;

    let sig = 0xf30a;
    if magic != sig {
        return Err(ext4_error(format!(
            "extent magic {magic:#x} is not {sig:#x}"
        )));
    }

    let (input, entries) = nom_u16(input, "extent entries are truncated")?;
    let (input, max_entries) = nom_u16(input, "extent max entries are truncated")?;
    let (input, depth) = nom_u16(input, "extent depth is truncated")?;
    let (_, generation) = nom_u32(input, "extent generation is truncated")?;

    Ok(ExtentHeader {
        entries,
        max_entries,
        depth,
        generation,
    })
}

fn parse_leaves(mut input: &[u8], block_count: u64) -> AccessorResult<Vec<Extent>> {
    let mut extents = Vec::new();
    let mut previous_end = None;

    while !input.is_empty() {
        let (remaining, logical_block) = nom_u32(input, "extent logical block is truncated")?;
        let (remaining, raw_length) = nom_u16(remaining, "extent length is truncated")?;
        let (remaining, start_hi) = nom_u16(remaining, "extent physical block high is truncated")?;
        let (remaining, start_lo) = nom_u32(remaining, "extent physical block is truncated")?;
        input = remaining;

        let (length, unwritten) = extent_length(raw_length)?;
        let physical_block = start_lo as u64 | (start_hi as u64) << 32;

        check_logical_range(logical_block, length)?;
        check_physical_range(physical_block, length as u64, block_count)?;

        let start = logical_block as u64;
        if let Some(end) = previous_end
            && start < end
        {
            return Err(ext4_error(format!(
                "extent at logical block {logical_block} overlaps the previous extent"
            )));
        }

        previous_end = Some(start + length as u64);

        extents.push(Extent {
            logical_block,
            length,
            physical_block,
            unwritten,
        })
    }

    Ok(extents)
}

fn parse_indexes(mut input: &[u8], block_count: u64) -> AccessorResult<Vec<ExtentIndex>> {
    let mut indexes = Vec::new();
    let mut previous = None;

    while !input.is_empty() {
        let (remaining, logical_block) = nom_u32(input, "extent index logical block is truncated")?;
        let (remaining, leaf_lo) = nom_u32(remaining, "extent index block is truncated")?;
        let (remaining, leaf_hi) = nom_u16(remaining, "extent index block high is truncated")?;
        let (remaining, _unused) = nom_u16(remaining, "extent index tail is truncated")?;
        input = remaining;

        let physical_block = leaf_lo as u64 | (leaf_hi as u64) << 32;
        check_physical_block(physical_block, block_count)?;

        if let Some(previous) = previous
            && logical_block <= previous
        {
            return Err(ext4_error(format!(
                "extent index {logical_block} is not after {previous}"
            )));
        }
        previous = Some(logical_block);

        indexes.push(ExtentIndex {
            logical_block,
            physical_block,
        })
    }

    Ok(indexes)
}

fn extent_length(raw: u16) -> AccessorResult<(u16, bool)> {
    if raw == 0 {
        return Err(ext4_error("extent length is 0"));
    }

    let max_len = 32768;
    if raw <= max_len {
        Ok((raw, false))
    } else {
        Ok((raw - max_len, true))
    }
}

fn check_logical_range(logical: u32, length: u16) -> AccessorResult<()> {
    let end = logical as u64 + length as u64;

    if end > u64::from(u32::MAX) + 1 {
        return Err(ext4_error(format!(
            "extent at logical block {logical} length {length} overflows"
        )));
    }

    Ok(())
}

fn check_physical_range(physical: u64, length: u64, block_count: u64) -> AccessorResult<()> {
    if physical == 0 {
        return Err(ext4_error("extent physical block is 0"));
    }

    let end = physical
        .checked_add(length)
        .ok_or_else(|| ext4_error("extent physical blocks overflow"))?;

    if end > block_count {
        return Err(ext4_error(format!(
            "extent physical blocks {physical}..{end} are outside the filesystem"
        )));
    }

    Ok(())
}

fn check_physical_block(physical: u64, block_count: u64) -> AccessorResult<()> {
    if physical == 0 || physical >= block_count {
        return Err(ext4_error(format!(
            "extent index block {physical} is outside the filesystem"
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{
        Extent, ExtentIndex, ExtentLimits, ExtentNode, extent_crc32c, parse_extent_node,
        verify_extent_block,
    };
    use crate::accessor::{
        error::AccessorError,
        filesystem::ext4::format::superblock::{SuperBlock, parse_superblock},
    };
    use std::{fs::read, path::PathBuf};

    fn limits(block_count: u64, capacity: usize, checksum_tail: bool) -> ExtentLimits {
        ExtentLimits {
            block_count,
            capacity,
            checksum_tail,
            expected_depth: None,
        }
    }

    fn put_u16(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn write_header(data: &mut [u8], entries: u16, max_entries: u16, depth: u16) {
        put_u16(data, 0, 0xf30a);
        put_u16(data, 2, entries);
        put_u16(data, 4, max_entries);
        put_u16(data, 6, depth);
    }

    fn write_leaf(data: &mut [u8], offset: usize, logical: u32, len: u16, physical: u64) {
        put_u32(data, offset, logical);
        put_u16(data, offset + 4, len);
        put_u16(data, offset + 6, (physical >> 32) as u16);
        put_u32(data, offset + 8, physical as u32);
    }

    fn write_index(data: &mut [u8], offset: usize, logical: u32, physical: u64) {
        put_u32(data, offset, logical);
        put_u32(data, offset + 4, physical as u32);
        put_u16(data, offset + 8, (physical >> 32) as u16);
    }

    fn superblock() -> SuperBlock {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ext4/debian/bookworm/firstblock.raw");
        parse_superblock(&read(path).unwrap()).unwrap()
    }

    fn ext4_reason(result: Result<impl std::fmt::Debug, AccessorError>) -> String {
        match result {
            Err(AccessorError::Ext4 { reason, .. }) => reason,
            other => panic!("expected an ext4 error, got {other:?}"),
        }
    }

    #[test]
    fn test_parse_extent_node_reads_a_leaf_root() {
        let mut data = vec![0_u8; 60];
        write_header(&mut data, 2, 4, 0);
        put_u32(&mut data, 8, 7);
        write_leaf(&mut data, 12, 0, 2, 40);
        write_leaf(&mut data, 24, 2, 1, 0x1_0000_0005);

        let node = parse_extent_node(&data, &limits(0x1_0000_0010, 60, false)).unwrap();
        let ExtentNode::Leaf { header, extents } = node else {
            panic!("expected a leaf");
        };

        assert_eq!(header.entries, 2);
        assert_eq!(header.max_entries, 4);
        assert_eq!(header.depth, 0);

        assert_eq!(header.generation, 7);
        assert_eq!(
            extents,
            vec![
                Extent {
                    logical_block: 0,
                    length: 2,
                    physical_block: 40,
                    unwritten: false,
                },
                Extent {
                    logical_block: 2,
                    length: 1,
                    physical_block: 0x1_0000_0005,
                    unwritten: false,
                },
            ]
        );
    }

    #[test]
    fn test_parse_extent_node_decodes_initialized_length_32768() {
        let mut data = vec![0_u8; 60];
        write_header(&mut data, 1, 4, 0);
        write_leaf(&mut data, 12, 0, 0x8000, 10);

        let node = parse_extent_node(&data, &limits(40_000, 60, false)).unwrap();
        let ExtentNode::Leaf { extents, .. } = node else {
            panic!("expected a leaf");
        };

        assert_eq!(extents[0].length, 32768);
        assert!(!extents[0].unwritten);
    }

    #[test]
    fn test_parse_extent_node_decodes_an_unwritten_extent() {
        let mut data = vec![0_u8; 60];
        write_header(&mut data, 1, 4, 0);
        write_leaf(&mut data, 12, 5, 0x8001, 10);

        let node = parse_extent_node(&data, &limits(100, 60, false)).unwrap();
        let ExtentNode::Leaf { extents, .. } = node else {
            panic!("expected a leaf");
        };

        assert_eq!(
            extents[0],
            Extent {
                logical_block: 5,
                length: 1,
                physical_block: 10,
                unwritten: true,
            }
        );
    }

    #[test]
    fn test_parse_extent_node_reads_an_index() {
        let mut data = vec![0_u8; 60];
        write_header(&mut data, 1, 4, 1);
        write_index(&mut data, 12, 0, 9);

        let node = parse_extent_node(&data, &limits(100, 60, false)).unwrap();

        assert_eq!(
            node,
            ExtentNode::Index {
                header: super::ExtentHeader {
                    entries: 1,
                    max_entries: 4,
                    depth: 1,
                    generation: 0,
                },
                indexes: vec![ExtentIndex {
                    logical_block: 0,
                    physical_block: 9,
                }],
            }
        );
    }

    #[test]
    fn test_parse_extent_node_rejects_a_zero_length() {
        let mut data = vec![0_u8; 60];
        write_header(&mut data, 1, 4, 0);
        write_leaf(&mut data, 12, 0, 0, 10);

        let reason = ext4_reason(parse_extent_node(&data, &limits(100, 60, false)));
        assert!(reason.contains("length is 0"));
    }

    #[test]
    fn test_parse_extent_node_rejects_an_overlap() {
        let mut data = vec![0_u8; 60];
        write_header(&mut data, 2, 4, 0);
        write_leaf(&mut data, 12, 0, 4, 10);
        write_leaf(&mut data, 24, 3, 1, 20);

        let reason = ext4_reason(parse_extent_node(&data, &limits(100, 60, false)));
        assert!(reason.contains("overlaps"));
    }

    #[test]
    fn test_parse_extent_node_rejects_entries_above_max() {
        let mut data = vec![0_u8; 60];
        write_header(&mut data, 2, 1, 0);

        let reason = ext4_reason(parse_extent_node(&data, &limits(100, 60, false)));
        assert!(reason.contains("exceed max"));
    }

    #[test]
    fn test_parse_extent_node_rejects_depth_above_the_walk_limit() {
        let mut data = vec![0_u8; 60];
        write_header(&mut data, 0, 4, 33);

        let reason = ext4_reason(parse_extent_node(&data, &limits(100, 60, false)));
        assert!(reason.contains("exceeds 32"));
    }

    #[test]
    fn test_parse_extent_node_rejects_a_physical_block_outside_the_filesystem() {
        let mut data = vec![0_u8; 60];
        write_header(&mut data, 1, 4, 0);
        write_leaf(&mut data, 12, 0, 4, 8);

        let reason = ext4_reason(parse_extent_node(&data, &limits(10, 60, false)));
        assert!(reason.contains("outside the filesystem"));
    }

    #[test]
    fn test_parse_extent_node_rejects_an_unsorted_index() {
        let mut data = vec![0_u8; 60];
        write_header(&mut data, 2, 4, 1);
        write_index(&mut data, 12, 5, 2);
        write_index(&mut data, 24, 5, 3);

        let reason = ext4_reason(parse_extent_node(&data, &limits(100, 60, false)));
        assert!(reason.contains("not after"));
    }

    #[test]
    fn test_verify_extent_block_checks_the_unused_slot() {
        let superblock = superblock();
        let seed = superblock.checksum_seed().unwrap();
        let mut data = vec![0_u8; 64];
        write_header(&mut data, 1, 2, 0);
        write_leaf(&mut data, 12, 0, 1, 4);

        let sum = extent_crc32c(seed, 11, 0xabcd, &data, 2);
        put_u32(&mut data, 36, sum);

        let node = parse_extent_node(&data, &limits(100, 64, true)).unwrap();
        verify_extent_block(&superblock, Some(seed), 11, 0xabcd, &data, node.header()).unwrap();

        data[24] ^= 0xff;
        let node = parse_extent_node(&data, &limits(100, 64, true)).unwrap();
        let reason = ext4_reason(verify_extent_block(
            &superblock,
            Some(seed),
            11,
            0xabcd,
            &data,
            node.header(),
        ));
        assert!(reason.contains("checksum mismatch"));
    }

    #[test]
    fn test_verify_extent_block_skips_checksums_when_metadata_csum_is_off() {
        let mut superblock = superblock();
        superblock.read_only_features &= !SuperBlock::METADATA_CSUM;
        let mut data = vec![0_u8; 64];
        write_header(&mut data, 1, 1, 0);
        write_leaf(&mut data, 12, 0, 1, 4);

        let node = parse_extent_node(&data, &limits(100, 64, false)).unwrap();
        verify_extent_block(&superblock, None, 11, 0xabcd, &data, node.header()).unwrap();
    }
}
