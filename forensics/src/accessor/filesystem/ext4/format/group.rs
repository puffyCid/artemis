use crate::{
    accessor::{
        error::AccessorResult,
        filesystem::ext4::format::{
            checksum::{crc16_check, crc32c_check},
            layout::Ext4Layout,
            superblock::{SuperBlock, combine_u16, combine_u32, ext4_error},
        },
    },
    utils::nom_helper::{nom_u16, nom_u32},
};

/// Block `GroupDescriptor` value
///
/// Each block group on the filesystem should have a group descriptor structure
///
/// <https://docs.kernel.org/filesystems/ext4/group_descr.html#block-group-descriptors>
#[derive(Debug)]
pub(crate) struct GroupDescriptor {
    /// Block number for of the block bitmap
    ///
    /// If 64 bit feature flag is enabled. Then this value is 8 bytes
    pub(crate) block_bitmap: u64,
    /// Block number for of the inode bitmap
    ///
    /// If 64 bit feature flag is enabled. Then this value is 8 bytes
    pub(crate) inode_bitmap: u64,
    /// Block number for of the inode tabe
    ///
    /// If 64 bit feature flag is enabled.
    /// Then this value requires upper and lower 32 bit values
    pub(crate) inode_table: u64,
    /// Free blocks count
    ///
    /// Requires lower and upper 16 bits counts
    pub(crate) free_blocks_count: u32,
    /// Free inodes count
    ///
    /// Requires lower and upper 16 bits counts
    pub(crate) free_inodes_count: u32,
    /// Directory count
    ///
    /// Requires lower and upper 16 bits counts
    pub(crate) used_dirs_count: u32,
    /// Block group flags
    ///
    /// <https://docs.kernel.org/filesystems/ext4/group_descr.html#bgflags>
    pub(crate) flags: u16,
    /// Location of the snapshot execlusion bitmap
    ///
    /// If 64 bit feature flag is enabled.
    /// Then this value requires upper and lower 32 bit values
    pub(crate) exclude_bitmap: u64,
    /// Block bitmap checksump.
    /// Requires lower and upper 16 bit values
    pub(crate) block_bitmap_csum: u32,
    /// Inode  bitmap checksump.
    /// Requires lower and uppper 16 bit values
    pub(crate) inode_bitmap_csum: u32,
    /// Unused inode count.
    /// Requires lower and upper 16 bit values
    pub(crate) unused_inodes_count: u32,
    /// `GroupDescriptor` CRC16 checksum
    pub(crate) checksum: u16,
}

impl GroupDescriptor {
    /// Flag that inode and bitmap table are not initalized
    pub(crate) const INODE_UNINIT: u16 = 0x1;
    /// Flag that block bitmap is not initialized
    pub(crate) const BLOCK_UNINIT: u16 = 0x2;
    /// Flag that inode table is zeroed
    pub(crate) const INODE_ZEROED: u16 = 0x4;
}

/// Perform CRC32C checksum check against `GroupDescriptor`
pub(crate) fn group_descriptor_crc32c(seed: u32, group: u32, data: &[u8]) -> u32 {
    let mut crc = crc32c_check(seed, &group.to_le_bytes());
    crc = crc32c_check(crc, &data[..30]);
    crc = crc32c_check(crc, &0_u16.to_le_bytes());

    if data.len() > 32 {
        crc = crc32c_check(crc, &data[32..]);
    }

    crc
}

/// Perform CRC16 checksum check against `GroupDescriptor`
pub(crate) fn group_descriptor_crc16(uuid: &[u8; 16], group: u32, data: &[u8]) -> u16 {
    let mut crc = crc16_check(0xffff, uuid);
    crc = crc16_check(crc, &group.to_le_bytes());
    crc = crc16_check(crc, &data[..30]);

    if data.len() > 32 {
        crc = crc16_check(crc, &data[32..]);
    }

    crc
}

/// Parse the group descriptor associated with an ext4 block
pub(crate) fn parse_group_descriptor(
    data: &[u8],
    descriptor_size: u16,
) -> AccessorResult<GroupDescriptor> {
    if descriptor_size < 32 || (descriptor_size > 32 && descriptor_size < 64) {
        return Err(ext4_error(format!(
            "group descriptor size {descriptor_size} is not 32 or at least 64"
        )));
    }

    if data.len() < descriptor_size as usize {
        return Err(ext4_error(format!(
            "group descriptor is {} bytes, expected {descriptor_size}",
            data.len()
        )));
    }

    let wide = descriptor_size >= 64;

    let (input, block_bitmap_lo) = nom_u32(data, "group descriptor block bitmap is truncated")?;
    let (input, inode_bitmap_lo) = nom_u32(input, "group descriptor inode bitmap is truncated")?;
    let (input, inode_table_lo) = nom_u32(input, "group descriptor inode table is truncated")?;

    let (input, free_blocks_lo) = nom_u16(input, "group descriptor free block count is truncated")?;
    let (input, free_inodes_lo) = nom_u16(input, "group descriptor free inode count is truncated")?;
    let (input, used_dirs_lo) = nom_u16(input, "group descriptor directory count is truncated")?;
    let (input, flags) = nom_u16(input, "group descriptor flags are truncated")?;

    let (input, exclude_bitmap_lo) =
        nom_u32(input, "group descriptor exclude bitmap is truncated")?;
    let (input, block_bitmap_csum_lo) =
        nom_u16(input, "group descriptor block bitmap checksum is truncated")?;
    let (input, inode_bitmap_csum_lo) =
        nom_u16(input, "group descriptor inode bitmap checksum is truncated")?;
    let (input, unused_inodes_lo) =
        nom_u16(input, "group descriptor unused inode count is truncated")?;
    let (input, checksum) = nom_u16(input, "group descriptor checksum is truncated")?;

    // If 64 bit feature flag is enabled
    // Get het upper 32 bit data
    let (
        block_bitmap_hi,
        inode_bitmap_hi,
        inode_table_hi,
        free_blocks_hi,
        free_inodes_hi,
        used_dirs_hi,
        unused_inodes_hi,
        exclude_bitmap_hi,
        block_bitmap_csum_hi,
        inode_bitmap_csum_hi,
    ) = if wide {
        let (input, block_bitmap_hi) =
            nom_u32(input, "group descriptor block bitmap high is truncated")?;
        let (input, inode_bitmap_hi) =
            nom_u32(input, "group descriptor inode bitmap high is truncated")?;
        let (input, inode_table_hi) =
            nom_u32(input, "group descriptor inode table high is truncated")?;
        let (input, free_blocks_hi) =
            nom_u16(input, "group descriptor free block count high is truncated")?;

        let (input, free_inodes_hi) =
            nom_u16(input, "group descriptor free inode count high is truncated")?;
        let (input, used_dirs_hi) =
            nom_u16(input, "group descriptor directory count high is truncated")?;
        let (input, unused_inodes_hi) = nom_u16(
            input,
            "group descriptor unused inode count high is truncated",
        )?;

        let (input, exclude_bitmap_hi) =
            nom_u32(input, "group descriptor exclude bitmap high is truncated")?;
        let (input, block_bitmap_csum_hi) = nom_u16(
            input,
            "group descriptor block bitmap checksum high is truncated",
        )?;
        let (input, inode_bitmap_csum_hi) = nom_u16(
            input,
            "group descriptor inode bitmap checksum high is truncated",
        )?;
        let (_input, _reserved) = nom_u32(input, "group descriptor reserved tail is truncated")?;

        (
            block_bitmap_hi,
            inode_bitmap_hi,
            inode_table_hi,
            free_blocks_hi,
            free_inodes_hi,
            used_dirs_hi,
            unused_inodes_hi,
            exclude_bitmap_hi,
            block_bitmap_csum_hi,
            inode_bitmap_csum_hi,
        )
    } else {
        (0, 0, 0, 0, 0, 0, 0, 0, 0, 0)
    };

    Ok(GroupDescriptor {
        block_bitmap: combine_u32(block_bitmap_lo, block_bitmap_hi, wide),
        inode_bitmap: combine_u32(inode_bitmap_lo, inode_bitmap_hi, wide),
        inode_table: combine_u32(inode_table_lo, inode_table_hi, wide),
        free_blocks_count: combine_u16(free_blocks_lo, free_blocks_hi, wide),
        free_inodes_count: combine_u16(free_inodes_lo, free_inodes_hi, wide),
        used_dirs_count: combine_u16(used_dirs_lo, used_dirs_hi, wide),
        flags,
        exclude_bitmap: combine_u32(exclude_bitmap_lo, exclude_bitmap_hi, wide),
        block_bitmap_csum: combine_u16(block_bitmap_csum_lo, block_bitmap_csum_hi, wide),
        inode_bitmap_csum: combine_u16(inode_bitmap_csum_lo, inode_bitmap_csum_hi, wide),
        unused_inodes_count: combine_u16(unused_inodes_lo, unused_inodes_hi, wide),
        checksum,
    })
}

/// Verify block group checksum if metadata checksum or group descriptors table checksum is enabled
pub(crate) fn verify_group_descriptor(
    superblock: &SuperBlock,
    checksum_seed: Option<u32>,
    group: u32,
    data: &[u8],
    descriptor: &GroupDescriptor,
) -> AccessorResult<()> {
    if data.len() < 32 {
        return Err(ext4_error(format!(
            "group {group} descriptor is {} bytes, expected at least 32",
            data.len()
        )));
    }

    let actual = if superblock.has_read_only(SuperBlock::METADATA_CSUM) {
        let seed = checksum_seed.ok_or_else(|| ext4_error("metadata checksum seed is missing"))?;
        (group_descriptor_crc32c(seed, group, data) & 0xffff) as u16
    } else if superblock.has_read_only(SuperBlock::GDT_CSUM) {
        group_descriptor_crc16(&superblock.uuid, group, data)
    } else {
        return Ok(());
    };

    if actual != descriptor.checksum {
        return Err(ext4_error(format!(
            "group {group} descriptor checksum mismatch: found {actual:#x}, stored {:#x}",
            descriptor.checksum
        )));
    }

    Ok(())
}

/// Validate `GroupDescriptor` values against the ext4 filesystem layout
pub(crate) fn validate_group_descriptor(
    layout: &Ext4Layout,
    superblock: &SuperBlock,
    group: u64,
    descriptor: &GroupDescriptor,
) -> AccessorResult<()> {
    let (first, last) = allowed_range(layout, superblock, group)?;
    let superblock_block = (layout.block_size == 1024) as u64;
    let gdt_first = layout.descriptor_table_block;
    let gdt_count = layout.descriptor_table_blocks as u64;

    if gdt_count == 0 {
        return Err(ext4_error("group descriptor table is empty"));
    }

    let gdt_last = gdt_first
        .checked_add(gdt_count - 1)
        .ok_or_else(|| ext4_error("group descriptor table block range overflows"))?;

    let mut span = Span {
        group,
        start: descriptor.block_bitmap,
        count: 1,
        first,
        last,
        superblock_block,
        gdt_first,
        gdt_last,
    };
    check_span("block bitmap", &span)?;

    span.start = descriptor.inode_bitmap;
    check_span("inode bitmap", &span)?;

    span.start = descriptor.inode_table;
    span.count = layout.inode_table_blocks as u64;
    check_span("inode table", &span)?;

    if descriptor.exclude_bitmap != 0 {
        span.start = descriptor.exclude_bitmap;
        span.count = 1;
        span.first = superblock.first_data_block as u64;
        span.last = layout.block_count - 1;

        check_span("exclude bitmap", &span)?;
    }

    Ok(())
}

/// Block layout range for a group
fn allowed_range(
    layout: &Ext4Layout,
    superblock: &SuperBlock,
    group: u64,
) -> AccessorResult<(u64, u64)> {
    let flex = superblock.has_incompatible(SuperBlock::FLEX_BG);
    let blocks_per_group = layout.blocks_per_group as u64;
    let origin = superblock.first_data_block as u64;

    let first = if flex {
        origin
    } else {
        let span = group
            .checked_mul(blocks_per_group)
            .ok_or_else(|| ext4_error(format!("block group {group} start block overflows")))?;

        origin
            .checked_add(span)
            .ok_or_else(|| ext4_error(format!("block group {group} start block overflows")))?
    };

    let last = if flex || group + 1 == layout.group_count {
        layout.block_count - 1
    } else {
        first
            .checked_add(blocks_per_group - 1)
            .ok_or_else(|| ext4_error(format!("block group {group} end block overflows")))?
    };

    if first > last {
        return Err(ext4_error(format!(
            "block group {group} starts at {first}, past the filesystem"
        )));
    }

    Ok((first, last))
}

/// Check span range for metadata blocks from the `GroupDescriptor`
struct Span {
    /// Metadata group
    ///
    /// Will be either `Block Bitmap`, `Inode Table`,
    /// `Inode Bitmap`, or `Exclude Bitmap`
    group: u64,
    /// Starting data block
    start: u64,
    /// Block count
    count: u64,
    /// First block
    first: u64,
    /// Last block
    last: u64,
    /// Superblock block number
    superblock_block: u64,
    /// First Group Descriptor table block
    gdt_first: u64,
    /// Last Group Descriptor table block
    gdt_last: u64,
}

/// Validate the location for metadata blocks
fn check_span(name: &str, span: &Span) -> AccessorResult<()> {
    if span.count == 0 {
        return Err(ext4_error(format!(
            "{name} for group {} is empty",
            span.group
        )));
    }

    let end = span.start.checked_add(span.count - 1).ok_or_else(|| {
        ext4_error(format!(
            "{name} for group {} at block {} overflows",
            span.group, span.start
        ))
    })?;

    if span.start < span.first || end > span.last {
        return Err(ext4_error(format!(
            "{name} for group {} is block {}, outside {}..={}",
            span.group, span.start, span.first, span.last
        )));
    }

    if span.start <= span.superblock_block && end >= span.superblock_block {
        return Err(ext4_error(format!(
            "{name} for group {} overlaps the superblock",
            span.group
        )));
    }

    if span.start <= span.gdt_last && end >= span.gdt_first {
        return Err(ext4_error(format!(
            "{name} for group {} overlaps the group descriptor table",
            span.group
        )));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{GroupDescriptor, parse_group_descriptor, verify_group_descriptor};
    use crate::accessor::{
        error::AccessorError,
        filesystem::ext4::format::{
            group::{group_descriptor_crc16, group_descriptor_crc32c},
            superblock::parse_superblock,
        },
    };
    use std::{fs::read, path::PathBuf};

    fn read_fixture(name: &str) -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ext4/debian/bookworm");
        path.push(name);
        read(path).unwrap()
    }

    fn group0() -> Vec<u8> {
        read_fixture("descriptor.raw")[..64].to_vec()
    }

    fn superblock() -> crate::accessor::filesystem::ext4::format::superblock::SuperBlock {
        parse_superblock(&read_fixture("firstblock.raw")).unwrap()
    }

    fn descriptor_group0() -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ext4/debian/bookworm/descriptor.raw");

        let data = read(path).unwrap();
        data[..64].to_vec()
    }

    fn bookwork_uuid() -> [u8; 16] {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ext4/debian/bookworm/firstblock.raw");

        let data = read(path).unwrap();
        data[0x68..0x78].try_into().unwrap()
    }

    #[test]
    fn test_parse_group_descriptor_debian12() {
        let desc = parse_group_descriptor(&group0(), 64).unwrap();

        assert_eq!(desc.block_bitmap, 1028);
        assert_eq!(desc.inode_bitmap, 1044);
        assert_eq!(desc.inode_table, 1060);
        assert_eq!(desc.free_blocks_count, 23541);
        assert_eq!(desc.free_inodes_count, 8140);
        assert_eq!(desc.used_dirs_count, 3);
        assert_eq!(desc.flags & GroupDescriptor::INODE_UNINIT, 0);
        assert_eq!(desc.flags & GroupDescriptor::BLOCK_UNINIT, 0);

        assert_eq!(desc.flags, GroupDescriptor::INODE_ZEROED);
        assert_eq!(desc.exclude_bitmap, 0);
        assert_eq!(desc.block_bitmap_csum, 3_965_864_587);
        assert_eq!(desc.inode_bitmap_csum, 973_384_871);
        assert_eq!(desc.unused_inodes_count, 8119);
        assert_eq!(desc.checksum, 61671);
    }

    #[test]
    fn test_parse_group_descriptor_32_byte_ignores_high_halves() {
        let mut data = group0();
        data[0x20..0x24].copy_from_slice(&0xffff_ffff_u32.to_le_bytes());

        let desc = parse_group_descriptor(&data[..32], 32).unwrap();
        assert_eq!(desc.block_bitmap, 1028);
        assert_eq!(desc.checksum, 61671);
    }

    #[test]
    fn test_verify_group_descriptor_debian12() {
        let superblock = superblock();
        let data = group0();
        let desc = parse_group_descriptor(&data, 64).unwrap();

        verify_group_descriptor(&superblock, superblock.checksum_seed(), 0, &data, &desc).unwrap();
    }

    #[test]
    fn test_verify_group_descriptor_rejects_modified_bytes() {
        let superblock = superblock();
        let mut data = group0();
        data[0] ^= 0xff;

        let desc = parse_group_descriptor(&data, 64).unwrap();
        let err = verify_group_descriptor(&superblock, superblock.checksum_seed(), 0, &data, &desc)
            .unwrap_err();

        assert!(matches!(
            err,
            AccessorError::Ext4 { reason, .. }
                if reason == "group 0 descriptor checksum mismatch: found 0xe675, stored 0xf0e7"
        ));
    }

    #[test]
    fn test_group_desc_crc32_debian() {
        let checksum = group_descriptor_crc32c(3502656522, 0, &descriptor_group0());
        assert_eq!(checksum & 0xffff, 61671);
    }

    #[test]
    fn test_group_desc_crc16_debian() {
        let checksum = group_descriptor_crc16(&bookwork_uuid(), 0, &descriptor_group0());
        assert_eq!(checksum, 41103);
    }
}
