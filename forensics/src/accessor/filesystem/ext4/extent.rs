use crate::accessor::{
    error::AccessorResult,
    filesystem::ext4::{
        format::{
            extent::{Extent, ExtentLimits, ExtentNode, parse_extent_node, verify_extent_block},
            inode::Inode,
            superblock::{SuperBlock, ext4_error},
        },
        volume::Ext4Volume,
    },
};
use std::io::{Read, Seek};

/// Mapping for one logical block lookup
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum ExtentRun {
    /// No allocated blocks within the `ExtentRun` range
    Hole {
        /// First logical block of the hole
        logical: u32,
        /// Logical blocks until the next `Extent`
        length: u64,
    },
    /// Initialized file data
    Data {
        /// First logical block of the data
        logical: u32,
        /// Physical block location of the data
        physical: u64,
        /// Blocks remaining in the `Extent`
        length: u16,
    },
    /// Allocated `Extent` but uninitialized blocks
    Unwritten {
        /// First logical block of value
        logical: u32,
        /// Physical block location of the data
        physical: u64,
        /// Blocks remaining in the `Extent`
        length: u16,
    },
}

/// Map the logical blocks through the `Extent` tree in the `Inode`
///
/// Requires `EXTENTS` flag and the `INLINE_DATA` flag must not be set
///
/// Searches the `Extent` data until it encounters a leaf value
pub(crate) fn map_extent<R>(
    volume: &Ext4Volume<R>,
    inode_number: u64,
    inode: &Inode,
    logical: u32,
) -> AccessorResult<ExtentRun>
where
    R: Read + Seek + Send,
{
    if inode.flags & Inode::EXTENTS == 0 {
        return Err(ext4_error(format!(
            "inode {inode_number} does not use extents"
        )));
    }

    if inode.flags & Inode::INLINE_DATA != 0 {
        return Err(ext4_error(format!(
            "inode {inode_number} has both extents and inline data"
        )));
    }

    let inode_number_u32 = u32::try_from(inode_number).map_err(|err| {
        ext4_error(format!(
            "inode {inode_number} does not fit in 32 bits: {err:?}"
        ))
    })?;

    let limit = ExtentLimits {
        block_count: volume.layout().block_count,
        capacity: inode.block_map.len(),
        checksum_tail: false,
        expected_depth: None,
    };

    let mut node = parse_extent_node(&inode.block_map, &limit)?;
    // Track child extents. Also has a max depth of 32 child extents
    let mut seen = Vec::new();

    loop {
        match node {
            // File data found
            ExtentNode::Leaf { extents, .. } => return map_leaf(&extents, logical),
            ExtentNode::Index { header, indexes } => {
                let Some(slot) = find_extent(indexes.len(), |index| {
                    indexes[index].logical_block <= logical
                }) else {
                    let next = indexes.first().map(|index| index.logical_block);
                    return hole_run(logical, next);
                };

                let child = indexes[slot].physical_block;
                // Check for recursive child extents (child points back to the paretn)
                // Max array size is 32
                if seen.contains(&child) {
                    return Err(ext4_error(format!(
                        "inode {inode_number} extent block {child} repeats in the tree"
                    )));
                }

                seen.push(child);
                node = read_extent_block(
                    volume,
                    inode_number_u32,
                    inode.generation,
                    child,
                    header.depth - 1,
                )?;
            }
        }
    }
}

/// Read the child `Extent` block
fn read_extent_block<R>(
    volume: &Ext4Volume<R>,
    inode_number: u32,
    inode_generation: u32,
    block: u64,
    expected_depth: u16,
) -> AccessorResult<ExtentNode>
where
    R: Read + Seek + Send,
{
    let data = volume.read_block(block)?;
    let checksum_tail = volume.superblock().has_read_only(SuperBlock::METADATA_CSUM);

    let limit = ExtentLimits {
        block_count: volume.layout().block_count,
        capacity: data.len(),
        checksum_tail,
        expected_depth: Some(expected_depth),
    };
    let node = parse_extent_node(&data, &limit)?;

    verify_extent_block(
        volume.superblock(),
        volume.checksum_seed(),
        inode_number,
        inode_generation,
        &data,
        node.header(),
    )?;

    Ok(node)
}

/// Return the `ExtentRun` for files
///
/// Get the logical and physical blocks for the `Extent` leaf
fn map_leaf(extents: &[Extent], logical: u32) -> AccessorResult<ExtentRun> {
    let Some(slot) = find_extent(extents.len(), |index| {
        extents[index].logical_block <= logical
    }) else {
        let next = extents.first().map(|extent| extent.logical_block);
        return hole_run(logical, next);
    };

    let extent = &extents[slot];
    let extent_end = extent.logical_block as u64 + extent.length as u64;

    // Leaf `Extent` has data
    if (logical as u64) < extent_end {
        let skip = u16::try_from(logical - extent.logical_block).map_err(|err| {
            ext4_error(format!(
                "extent offset at logical block {logical} does not fit: {err:?}"
            ))
        })?;

        let length = extent.length - skip;
        let physical = extent
            .physical_block
            .checked_add(u64::from(skip))
            .ok_or_else(|| ext4_error("extent physical block overflows"))?;

        let run = if extent.unwritten {
            ExtentRun::Unwritten {
                logical,
                physical,
                length,
            }
        } else {
            ExtentRun::Data {
                logical,
                physical,
                length,
            }
        };

        return Ok(run);
    }

    // Possible sparse data encountered
    let next = extents.get(slot + 1).map(|extent| extent.logical_block);
    hole_run(logical, next)
}

/// Entry where the key is less than or equal to provided value
///
/// Searches the `Extent` array for the last entry associated within the provided array length
fn find_extent(length: usize, is_candidate: impl Fn(usize) -> bool) -> Option<usize> {
    let mut low = 0;
    let mut high = length;

    while low < high {
        let mid = low + (high - low) / 2;
        if is_candidate(mid) {
            low = mid + 1;
        } else {
            high = mid;
        }
    }

    low.checked_sub(1)
}

/// Calculates holes in blocks
///
/// If the current block is 1 and the next block is 10. Then 9 blocks are related to unallocated data
fn hole_run(logical: u32, next: Option<u32>) -> AccessorResult<ExtentRun> {
    let length = match next {
        Some(start) if start > logical => u64::from(start - logical),
        Some(start) => {
            return Err(ext4_error(format!(
                "extent at logical block {start} overlaps {logical}"
            )));
        }
        None => u64::from(u32::MAX - logical) + 1,
    };

    Ok(ExtentRun::Hole { logical, length })
}

#[cfg(test)]
mod tests {
    use super::{ExtentRun, map_extent};
    use crate::accessor::{
        error::AccessorError,
        filesystem::ext4::{
            format::{
                checksum::crc32c_check,
                extent::extent_crc32c,
                inode::{Inode, parse_inode},
                superblock::SuperBlock,
            },
            volume::Ext4Volume,
        },
    };
    use std::{fs::read, io::Cursor, path::PathBuf};

    fn fixture() -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ext4/debian/bookworm/firstblock.raw");
        read(path).unwrap()
    }

    fn group0() -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ext4/debian/bookworm/descriptor.raw");
        read(path).unwrap()[..64].to_vec()
    }

    fn restore_checksum(superblock: &mut [u8]) {
        let sum = crc32c_check(0xffff_ffff, &superblock[..0x3fc]);
        superblock[0x3fc..0x400].copy_from_slice(&sum.to_le_bytes());
    }

    fn one_group_superblock() -> Vec<u8> {
        let mut superblock = fixture();
        superblock[0x00..0x04].copy_from_slice(&8160_u32.to_le_bytes());
        superblock[0x04..0x08].copy_from_slice(&32768_u32.to_le_bytes());

        restore_checksum(&mut superblock);
        superblock
    }

    fn one_group_image(descriptor: &[u8]) -> Cursor<Vec<u8>> {
        let superblock = one_group_superblock();
        let mut image = vec![0_u8; 8192];
        image[1024..2048].copy_from_slice(&superblock);

        image[4096..4096 + descriptor.len()].copy_from_slice(descriptor);
        Cursor::new(image)
    }

    fn open_volume() -> Ext4Volume<Cursor<Vec<u8>>> {
        Ext4Volume::open(one_group_image(&group0()), "bookworm").unwrap()
    }

    fn open_with_block(block: u64, contents: &[u8]) -> Ext4Volume<Cursor<Vec<u8>>> {
        let mut image = one_group_image(&group0()).into_inner();
        let offset = usize::try_from(block).unwrap() * 4096;
        let end = offset + contents.len();
        if image.len() < end {
            image.resize(end, 0);
        }
        image[offset..end].copy_from_slice(contents);
        let volume = Ext4Volume::open(Cursor::new(image), "bookworm").unwrap();
        assert_eq!(volume.layout().block_size, 4096);
        volume
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

    fn inode_with_tree(tree: &[u8]) -> Inode {
        assert!(tree.len() <= 60);
        let mut data = vec![0_u8; 256];
        data[0..2].copy_from_slice(&0x8000_u16.to_le_bytes());
        data[0x20..0x24].copy_from_slice(&Inode::EXTENTS.to_le_bytes());

        data[0x28..0x28 + tree.len()].copy_from_slice(tree);
        data[0x64..0x68].copy_from_slice(&0xabcd_u32.to_le_bytes());
        parse_inode(&data, 256).unwrap()
    }

    fn leaf_root(logical: u32, len: u16, physical: u64) -> Vec<u8> {
        let mut data = vec![0_u8; 60];
        write_header(&mut data, 1, 4, 0);
        write_leaf(&mut data, 12, logical, len, physical);
        data
    }

    fn seal_block(data: &mut [u8], max_entries: u16, seed: u32, generation: u32) {
        let sum = extent_crc32c(seed, 1, generation, data, max_entries);
        let tail = 12 + usize::from(max_entries) * 12;
        put_u32(data, tail, sum);
    }

    fn ext4_reason(result: Result<impl std::fmt::Debug, AccessorError>) -> String {
        match result {
            Err(AccessorError::Ext4 { reason, .. }) => reason,
            other => panic!("expected an ext4 error, got {other:?}"),
        }
    }

    #[test]
    fn test_map_extent_returns_data_holes_and_the_remainder() {
        let volume = open_volume();
        let inode = inode_with_tree(&leaf_root(10, 4, 50));

        assert_eq!(
            map_extent(&volume, 1, &inode, 0).unwrap(),
            ExtentRun::Hole {
                logical: 0,
                length: 10
            }
        );
        assert_eq!(
            map_extent(&volume, 1, &inode, 10).unwrap(),
            ExtentRun::Data {
                logical: 10,
                physical: 50,
                length: 4
            }
        );

        assert_eq!(
            map_extent(&volume, 1, &inode, 12).unwrap(),
            ExtentRun::Data {
                logical: 12,
                physical: 52,
                length: 2
            }
        );
        assert_eq!(
            map_extent(&volume, 1, &inode, 14).unwrap(),
            ExtentRun::Hole {
                logical: 14,
                length: u64::from(u32::MAX - 14) + 1,
            }
        );
    }

    #[test]
    fn test_map_extent_returns_the_unwritten_remainder() {
        let volume = open_volume();
        let inode = inode_with_tree(&leaf_root(0, 0x8002, 80));

        assert_eq!(
            map_extent(&volume, 1, &inode, 1).unwrap(),
            ExtentRun::Unwritten {
                logical: 1,
                physical: 81,
                length: 1
            }
        );
    }

    #[test]
    fn test_map_extent_treats_an_empty_leaf_as_a_hole() {
        let volume = open_volume();
        let mut root = vec![0_u8; 60];
        write_header(&mut root, 0, 4, 0);
        let inode = inode_with_tree(&root);

        assert_eq!(
            map_extent(&volume, 1, &inode, 0).unwrap(),
            ExtentRun::Hole {
                logical: 0,
                length: u64::from(u32::MAX) + 1,
            }
        );
    }

    #[test]
    fn test_map_extent_rejects_an_inode_without_extents() {
        let volume = open_volume();
        let mut inode = inode_with_tree(&leaf_root(0, 1, 4));
        inode.flags &= !Inode::EXTENTS;
        let reason = ext4_reason(map_extent(&volume, 1, &inode, 0));

        assert!(reason.contains("does not use extents"));
    }

    #[test]
    fn test_map_extent_rejects_inline_data() {
        let volume = open_volume();
        let mut inode = inode_with_tree(&leaf_root(0, 1, 4));
        inode.flags |= Inode::INLINE_DATA;
        let reason = ext4_reason(map_extent(&volume, 1, &inode, 0));

        assert!(reason.contains("inline data"));
    }

    #[test]
    fn test_map_extent_reads_a_depth_one_leaf() {
        let mut leaf = vec![0_u8; 4096];
        write_header(&mut leaf, 1, 1, 0);
        write_leaf(&mut leaf, 12, 0, 8, 20);

        let volume = open_volume();
        seal_block(&mut leaf, 1, volume.checksum_seed().unwrap(), 0xabcd);
        let volume = open_with_block(3, &leaf);

        let mut root = vec![0_u8; 60];
        write_header(&mut root, 1, 1, 1);
        write_index(&mut root, 12, 0, 3);
        let inode = inode_with_tree(&root);

        assert_eq!(
            map_extent(&volume, 1, &inode, 3).unwrap(),
            ExtentRun::Data {
                logical: 3,
                physical: 23,
                length: 5
            }
        );
    }

    #[test]
    fn test_map_extent_stops_before_an_index_key() {
        let volume = open_volume();
        let mut root = vec![0_u8; 60];
        write_header(&mut root, 1, 1, 1);
        write_index(&mut root, 12, 10, 50);

        let inode = inode_with_tree(&root);

        assert_eq!(
            map_extent(&volume, 1, &inode, 0).unwrap(),
            ExtentRun::Hole {
                logical: 0,
                length: 10
            }
        );
    }

    #[test]
    fn test_map_extent_rejects_a_bad_child_checksum() {
        let mut leaf = vec![0_u8; 4096];
        write_header(&mut leaf, 1, 1, 0);
        write_leaf(&mut leaf, 12, 0, 1, 20);
        let volume = open_with_block(3, &leaf);

        let mut root = vec![0_u8; 60];
        write_header(&mut root, 1, 1, 1);
        write_index(&mut root, 12, 0, 3);

        let inode = inode_with_tree(&root);
        let reason = ext4_reason(map_extent(&volume, 1, &inode, 0));

        assert!(reason.contains("checksum mismatch"));
    }

    #[test]
    fn test_map_extent_rejects_a_child_depth_mismatch() {
        let mut child = vec![0_u8; 4096];
        write_header(&mut child, 1, 1, 2);
        write_index(&mut child, 12, 0, 4);

        let volume = open_volume();
        seal_block(&mut child, 1, volume.checksum_seed().unwrap(), 0xabcd);
        let volume = open_with_block(3, &child);

        let mut root = vec![0_u8; 60];
        write_header(&mut root, 1, 1, 1);
        write_index(&mut root, 12, 0, 3);

        let inode = inode_with_tree(&root);
        let reason = ext4_reason(map_extent(&volume, 1, &inode, 0));
        assert!(reason.contains("does not match"));
    }

    #[test]
    fn test_map_extent_rejects_a_repeated_block() {
        let mut child = vec![0_u8; 4096];
        write_header(&mut child, 1, 1, 1);
        write_index(&mut child, 12, 0, 3);

        let volume = open_volume();
        seal_block(&mut child, 1, volume.checksum_seed().unwrap(), 0xabcd);
        let volume = open_with_block(3, &child);

        let mut root = vec![0_u8; 60];
        write_header(&mut root, 1, 1, 2);
        write_index(&mut root, 12, 0, 3);

        let inode = inode_with_tree(&root);
        let reason = ext4_reason(map_extent(&volume, 1, &inode, 0));
        assert!(reason.contains("repeats"));
    }

    #[test]
    fn test_map_extent_skips_checksums_when_metadata_csum_is_off() {
        let mut leaf = vec![0_u8; 4096];
        write_header(&mut leaf, 1, 1, 0);
        write_leaf(&mut leaf, 12, 0, 1, 20);

        let mut volume = open_with_block(3, &leaf);
        volume.superblock.read_only_features &= !SuperBlock::METADATA_CSUM;

        let mut root = vec![0_u8; 60];
        write_header(&mut root, 1, 1, 1);
        write_index(&mut root, 12, 0, 3);
        let inode = inode_with_tree(&root);

        assert_eq!(
            map_extent(&volume, 1, &inode, 0).unwrap(),
            ExtentRun::Data {
                logical: 0,
                physical: 20,
                length: 1
            }
        );
    }
}
