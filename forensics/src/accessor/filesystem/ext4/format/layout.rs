use crate::accessor::{
    error::AccessorResult,
    filesystem::ext4::format::superblock::{SuperBlock, ext4_error},
};

/// Location where `Superblock` and group descriptors are stored
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum SuperBlockCopies {
    /// Every group stores a copy
    EveryGroup,
    /// Group 0, 1, and groups that are powers of 3, 5, or 7
    Sparse,
    /// Group 0 and the two backup groups in the `SuperBlock`
    Sparse2([u32; 2]),
}

/// Block, group, and inode table data
/// derived from a `SuperBlock` value
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Ext4Layout {
    /// Bytes per block
    /// Should be `1024 << log_block_size`
    pub(crate) block_size: u32,
    /// Total blocks, including the high half when
    /// 64-bit mode is active
    pub(crate) block_count: u64,
    /// Number of block groups
    pub(crate) group_count: u64,
    /// Blocks in a complete group
    pub(crate) blocks_per_group: u32,
    /// Inodes in every group
    pub(crate) inodes_per_group: u32,
    /// Bytes per inode
    pub(crate) inode_size: u16,
    /// Bytes per group descriptor
    pub(crate) descriptor_size: u16,
    /// First block of the primary group descriptor table
    pub(crate) descriptor_table_block: u64,
    /// Blocks occupied by the primary group descriptor table
    pub(crate) descriptor_table_blocks: u32,
    /// Blocks by one group's inode table
    pub(crate) inode_table_blocks: u32,
    /// Groups in one flex group
    pub(crate) groups_per_flex: u32,
    /// Descriptor blocks reserved for growth
    pub(crate) reserved_gdt_blocks: u16,
    /// Location of `SuperBlock` and group descriptor copies
    pub(crate) copies: SuperBlockCopies,
}

impl Ext4Layout {
    /// Return `Ext4Layout` from a `SuperBlock` value
    pub(crate) fn from_superblock(superblock: &SuperBlock) -> AccessorResult<Self> {
        let dynamic = superblock.revision >= 1;
        let block_size = block_size(superblock.log_block_size)?;
        let blocks_per_group = superblock.blocks_per_group;
        let inodes_per_group = superblock.inodes_per_group;

        if blocks_per_group == 0 {
            return Err(ext4_error("blocks per group is 0"));
        }

        if blocks_per_group as u64 > (block_size * 8) as u64 {
            return Err(ext4_error(format!(
                "blocks per group {blocks_per_group} exceeds the block bitmap"
            )));
        }

        if inodes_per_group == 0 || inodes_per_group as u64 > (block_size as u64 * 8) {
            return Err(ext4_error(format!(
                "inodes per group {inodes_per_group} exceeds the inode bitmap"
            )));
        }

        // Block size should be 1024 and first block should start at it too
        let expected_first = (block_size == 1024) as u32;
        if superblock.first_data_block != expected_first {
            return Err(ext4_error(format!(
                "first data block is {}, expected {expected_first} for {block_size}-byte blocks",
                superblock.first_data_block
            )));
        }

        if dynamic && !superblock.has_read_only(SuperBlock::BIGALLOC) {
            if superblock.log_cluster_size != superblock.log_block_size
                || superblock.clusters_per_group != blocks_per_group
            {
                return Err(ext4_error(
                    "cluster size differs from block size without bigalloc",
                ));
            }
        }

        let block_count = if dynamic {
            superblock.blocks_count()
        } else {
            superblock.blocks_count_low as u64
        };

        if block_count <= superblock.first_data_block as u64 {
            return Err(ext4_error(format!(
                "block count {block_count} does not extend past the first data block"
            )));
        }

        let group_count =
            (block_count - superblock.first_data_block as u64 - 1) / blocks_per_group as u64 + 1;

        let inode_count = group_count * inodes_per_group as u64;
        if inode_count != superblock.inodes_count as u64 {
            return Err(ext4_error(format!(
                "inode count {} does not match {group_count} groups of {inodes_per_group}",
                superblock.inodes_count
            )));
        }

        let inode_size = if dynamic { superblock.inode_size } else { 128 };
        if !inode_size.is_power_of_two() || inode_size < 128 || inode_size as u32 > block_size {
            return Err(ext4_error(format!(
                "inode size {inode_size} is outside 128..={block_size} or is not a power of two"
            )));
        }

        let inode_bytes = inodes_per_group as u64 * inode_size as u64;
        if inode_bytes % block_size as u64 != 0 {
            return Err(ext4_error(
                "inode table does not occupy a whole number of blocks",
            ));
        }

        let inode_table_blocks = u32::try_from(inode_bytes / block_size as u64)
            .map_err(|_| ext4_error("inode table block count does not fit in 32 bits"))?;
        let bit64 = dynamic && superblock.has_incompatible(SuperBlock::BIT64);
        let descriptor_size = if bit64 {
            superblock.descriptor_size
        } else {
            32
        };

        let max_descriptor = u16::try_from(block_size.min(1024)).unwrap_or(1024);
        if bit64
            && (!descriptor_size.is_power_of_two()
                || descriptor_size < 64
                || descriptor_size > max_descriptor)
        {
            return Err(ext4_error(format!(
                "group descriptor size {descriptor_size} is outside 64..={max_descriptor} or is not a power of two"
            )));
        }

        let descriptor_bytes = group_count * descriptor_size as u64;
        let descriptor_table_blocks = descriptor_bytes.div_ceil(block_size as u64);
        let descriptor_table_blocks = u32::try_from(descriptor_table_blocks).map_err(|_| {
            ext4_error("group descriptor table block count does not fit in 32 bits")
        })?;

        let groups_per_flex = if dynamic && superblock.has_incompatible(SuperBlock::FLEX_BG) {
            if superblock.log_groups_per_flex > 31 {
                return Err(ext4_error(format!(
                    "flex group shift {} does not fit in 32 bits",
                    superblock.log_groups_per_flex
                )));
            }
            1_u32 << superblock.log_groups_per_flex
        } else {
            1
        };

        let reserved_gdt_blocks = if dynamic && superblock.has_compatible(SuperBlock::RESIZE_INODE)
        {
            superblock.reserved_gdt_blocks
        } else {
            0
        };

        let copies = superblock_copies(superblock, dynamic);

        Ok(Self {
            block_size,
            block_count,
            group_count,
            blocks_per_group,
            inodes_per_group,
            inode_size,
            descriptor_size,
            descriptor_table_block: superblock.first_data_block as u64 + 1,
            descriptor_table_blocks,
            inode_table_blocks,
            groups_per_flex,
            reserved_gdt_blocks,
            copies,
        })
    }

    /// Return if group stores a `SuperBlock` and group descriptor copy
    pub(crate) fn has_superblock(&self, group: u32) -> bool {
        if group == 0 {
            return true;
        }

        match self.copies {
            SuperBlockCopies::EveryGroup => true,
            SuperBlockCopies::Sparse => group == 1 || is_sparse_group(group),
            SuperBlockCopies::Sparse2([first, second]) => group == first || group == second,
        }
    }
}

/// Determine block size
fn block_size(log_block_size: u32) -> AccessorResult<u32> {
    if log_block_size > 6 {
        return Err(ext4_error(format!(
            "log block size {log_block_size} is outside 0..=6"
        )));
    }

    Ok(1024 << log_block_size)
}

/// Determine `SuperBlockCopies` value
fn superblock_copies(superblock: &SuperBlock, dynamic: bool) -> SuperBlockCopies {
    if !dynamic {
        return SuperBlockCopies::EveryGroup;
    }

    if superblock.has_compatible(SuperBlock::SPARSE_SUPER2) {
        return SuperBlockCopies::Sparse2(superblock.backup_block_groups);
    }

    if superblock.has_read_only(SuperBlock::SPARSE_SUPER) {
        return SuperBlockCopies::Sparse;
    }

    SuperBlockCopies::EveryGroup
}

/// Check if group is sparse
fn is_sparse_group(group: u32) -> bool {
    is_power(group, 3) || is_power(group, 5) || is_power(group, 7)
}

/// Check if group is power of base value
fn is_power(mut group: u32, base: u32) -> bool {
    if group < base {
        return false;
    }

    while group > base {
        if group % base != 0 {
            return false;
        }

        group /= base;
    }

    group == base
}

#[cfg(test)]
mod tests {
    use super::Ext4Layout;
    use crate::accessor::{
        error::{AccessorError, AccessorResult},
        filesystem::ext4::format::superblock::{SuperBlock, parse_superblock},
    };
    use std::{fs::read, path::PathBuf};

    fn fixture() -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ext4/debian/bookworm/firstblock.raw");
        read(path).unwrap()
    }

    fn layout_of(data: &[u8]) -> Ext4Layout {
        Ext4Layout::from_superblock(&parse_superblock(data).unwrap()).unwrap()
    }

    fn ext4_reason<T>(result: AccessorResult<T>) -> String {
        match result {
            Err(AccessorError::Ext4 { reason, .. }) => reason,
            Err(err) => panic!("expected EXT4 error, got {err}"),
            Ok(_) => panic!("expected EXT4 error"),
        }
    }

    #[test]
    fn test_layout_debian12() {
        let layout = layout_of(&fixture());

        assert_eq!(layout.block_size, 4096);
        assert_eq!(layout.block_count, 6103296);
        assert_eq!(layout.group_count, 187);

        assert_eq!(layout.blocks_per_group, 32768);
        assert_eq!(layout.inodes_per_group, 8160);
        assert_eq!(layout.inode_size, 256);
        assert_eq!(layout.descriptor_size, 64);

        assert_eq!(layout.descriptor_table_block, 1);
        assert_eq!(layout.descriptor_table_blocks, 3);
        assert_eq!(layout.inode_table_blocks, 510);
        assert_eq!(layout.groups_per_flex, 16);
        assert_eq!(layout.reserved_gdt_blocks, 1024);

        assert!(layout.has_superblock(0));
        assert!(layout.has_superblock(1));
        assert!(layout.has_superblock(3));
        assert!(layout.has_superblock(5));

        assert!(layout.has_superblock(7));
        assert!(layout.has_superblock(9));
        assert!(!layout.has_superblock(2));
        assert!(!layout.has_superblock(11));
    }

    #[test]
    fn test_revision_0_uses_fixed_sizes() {
        let mut data = fixture();
        data[0x4c..0x50].copy_from_slice(&0_u32.to_le_bytes());
        let layout = layout_of(&data);

        assert_eq!(layout.inode_size, 128);
        assert_eq!(layout.descriptor_size, 32);
        assert_eq!(layout.groups_per_flex, 1);
        assert_eq!(layout.reserved_gdt_blocks, 0);
        assert_eq!(layout.inode_table_blocks, 255);
        assert_eq!(layout.descriptor_table_blocks, 2);
        assert!(layout.has_superblock(2));
    }

    #[test]
    fn test_high_block_count_is_ignored_without_64bit() {
        let mut data = fixture();
        let features =
            u32::from_le_bytes(data[0x60..0x64].try_into().unwrap()) & !SuperBlock::BIT64;
        data[0x60..0x64].copy_from_slice(&features.to_le_bytes());
        data[0x150..0x154].copy_from_slice(&2_u32.to_le_bytes());
        let layout = layout_of(&data);

        assert_eq!(layout.block_count, 6103296);
        assert_eq!(layout.group_count, 187);
        assert_eq!(layout.descriptor_size, 32);
        assert_eq!(layout.descriptor_table_blocks, 2);
    }

    #[test]
    fn test_flex_bg_off_ignores_log_groups_per_flex() {
        let mut data = fixture();
        let features =
            u32::from_le_bytes(data[0x60..0x64].try_into().unwrap()) & !SuperBlock::FLEX_BG;
        data[0x60..0x64].copy_from_slice(&features.to_le_bytes());

        assert_eq!(layout_of(&data).groups_per_flex, 1);
    }

    #[test]
    fn test_resize_inode_off_ignores_reserved_gdt_blocks() {
        let mut data = fixture();
        let features =
            u32::from_le_bytes(data[0x5c..0x60].try_into().unwrap()) & !SuperBlock::RESIZE_INODE;
        data[0x5c..0x60].copy_from_slice(&features.to_le_bytes());

        assert_eq!(layout_of(&data).reserved_gdt_blocks, 0);
    }

    #[test]
    fn test_sparse_super2_replaces_sparse_super() {
        let mut data = fixture();
        let compat =
            u32::from_le_bytes(data[0x5c..0x60].try_into().unwrap()) | SuperBlock::SPARSE_SUPER2;
        data[0x5c..0x60].copy_from_slice(&compat.to_le_bytes());
        data[0x24c..0x250].copy_from_slice(&4_u32.to_le_bytes());
        data[0x250..0x254].copy_from_slice(&11_u32.to_le_bytes());
        let layout = layout_of(&data);

        assert!(layout.has_superblock(0));
        assert!(layout.has_superblock(4));

        assert!(layout.has_superblock(11));
        assert!(!layout.has_superblock(1));
        assert!(!layout.has_superblock(3));
    }

    #[test]
    fn test_layout_rejects_zero_blocks_per_group() {
        let mut data = fixture();
        data[0x20..0x24].copy_from_slice(&0_u32.to_le_bytes());

        let reason = ext4_reason(Ext4Layout::from_superblock(
            &parse_superblock(&data).unwrap(),
        ));
        assert!(reason.contains("blocks per group is 0"));
    }

    #[test]
    fn test_layout_rejects_blocks_per_group_past_bitmap() {
        let mut data = fixture();
        data[0x20..0x24].copy_from_slice(&32769_u32.to_le_bytes());

        let reason = ext4_reason(Ext4Layout::from_superblock(
            &parse_superblock(&data).unwrap(),
        ));
        assert!(reason.contains("block bitmap"));
    }

    #[test]
    fn test_layout_rejects_bad_first_data_block() {
        let mut data = fixture();
        data[0x14..0x18].copy_from_slice(&1_u32.to_le_bytes());

        let reason = ext4_reason(Ext4Layout::from_superblock(
            &parse_superblock(&data).unwrap(),
        ));
        assert!(reason.contains("first data block"));
    }

    #[test]
    fn test_layout_rejects_log_block_size_past_64k() {
        let mut data = fixture();
        data[0x18..0x1c].copy_from_slice(&7_u32.to_le_bytes());

        let reason = ext4_reason(Ext4Layout::from_superblock(
            &parse_superblock(&data).unwrap(),
        ));
        assert!(reason.contains("log block size"));
    }

    #[test]
    fn test_layout_rejects_cluster_mismatch_without_bigalloc() {
        let mut data = fixture();
        data[0x1c..0x20].copy_from_slice(&3_u32.to_le_bytes());

        let reason = ext4_reason(Ext4Layout::from_superblock(
            &parse_superblock(&data).unwrap(),
        ));
        assert!(reason.contains("cluster size"));
    }

    #[test]
    fn test_layout_rejects_inode_size_that_is_not_a_power_of_two() {
        let mut data = fixture();
        data[0x58..0x5a].copy_from_slice(&200_u16.to_le_bytes());

        let reason = ext4_reason(Ext4Layout::from_superblock(
            &parse_superblock(&data).unwrap(),
        ));
        assert!(reason.contains("inode size"));
    }

    #[test]
    fn test_layout_rejects_descriptor_size_that_is_not_a_power_of_two() {
        let mut data = fixture();
        data[0xfe..0x100].copy_from_slice(&96_u16.to_le_bytes());

        let reason = ext4_reason(Ext4Layout::from_superblock(
            &parse_superblock(&data).unwrap(),
        ));
        assert!(reason.contains("group descriptor size"));
    }

    #[test]
    fn test_layout_rejects_inode_count_mismatch() {
        let mut data = fixture();
        data[0x00..0x04].copy_from_slice(&1_u32.to_le_bytes());

        let reason = ext4_reason(Ext4Layout::from_superblock(
            &parse_superblock(&data).unwrap(),
        ));
        assert!(reason.contains("inode count"));
    }
}
