use crate::accessor::{
    error::{AccessorError, AccessorResult},
    filesystem::ext4::format::{
        group::{
            GroupDescriptor, parse_group_descriptor, validate_group_descriptor,
            verify_group_descriptor,
        },
        inode::{Inode, parse_inode, verify_inode},
        layout::Ext4Layout,
        superblock::{SuperBlock, ext4_error, parse_superblock},
    },
};
use std::{
    io::{Read, Seek, SeekFrom},
    sync::{Mutex, MutexGuard},
};

/// Unique Inode and generation for a filesystem entry
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Ext4EntryRef {
    /// Inode value
    pub(crate) inode: u64,
    /// Generation value for the inode
    pub(crate) generation: u32,
}

/// Open EXT4 filesystem
#[derive(Debug)]
pub(crate) struct Ext4Volume<R: Read + Seek + Send> {
    /// Path to ext4 filesystem
    pub(crate) target_path: String,
    /// Parsed `SuperBlock` structure
    pub(crate) superblock: SuperBlock,
    /// Parsed ext4 filesystem layout
    pub(crate) layout: Ext4Layout,
    /// Seed for the checksum metadata
    pub(crate) checksum_seed: Option<u32>,
    /// Primary `Group Descriptor` table
    pub(crate) groups: Vec<GroupDescriptor>,
    /// Reader for the ext4 filesystem
    pub(crate) reader: Mutex<R>,
}

impl<R: Read + Seek + Send> Ext4Volume<R> {
    /// Largest byte range `read_blocks` or `read_group_descriptors` can allocate
    ///
    /// Prevents large allocations from a corrupted `SuperBlock`
    const MAX_READ_BYTES: u64 = 16 * 1024 * 1024;

    /// Use the provided reader to read and parse the `SuperBlock` bytes
    pub(crate) fn open(mut reader: R, label: impl Into<String>) -> AccessorResult<Self> {
        let target_path = label.into();

        let superblock_offset = 1024;
        reader
            .seek(SeekFrom::Start(superblock_offset))
            .map_err(|err| {
                ext4_path_error(
                    &target_path,
                    format!("failed to seek to the superblock: {err}"),
                )
            })?;

        let mut data = [0_u8; 1024];
        reader.read_exact(&mut data).map_err(|err| {
            ext4_path_error(
                &target_path,
                format!("failed to read the superblock: {err}"),
            )
        })?;

        let superblock = parse_superblock(&data)?;
        superblock.validate_superblock(&data)?;

        let layout = Ext4Layout::from_superblock(&superblock)?;
        reject_unsupported(&superblock)?;
        let checksum_seed = superblock.checksum_seed();

        let mut volume = Self {
            target_path,
            superblock,
            layout,
            checksum_seed,
            groups: Vec::new(),
            reader: Mutex::new(reader),
        };
        volume.groups = volume.read_group_descriptors()?;

        Ok(volume)
    }

    /// Absolute byte offset for an inode value
    pub(crate) fn inode_offset(&self, inode: u64) -> AccessorResult<u64> {
        let count = self.superblock.inodes_count as u64;
        if inode == 0 || inode > count {
            return Err(ext4_error(format!("inode {inode} is outside 1..={count}")));
        }

        let per_group = self.layout.inodes_per_group as u64;
        let group = (inode - 1) / per_group;
        let index = (inode - 1) % per_group;

        let table = self.group_descriptors(group)?.inode_table;
        let inode_size = self.layout.inode_size as u64;

        let record = index
            .checked_mul(inode_size)
            .ok_or_else(|| ext4_error(format!("inode {inode} offset overflows")))?;

        let table_bytes = (self.layout.inode_table_blocks as u64)
            .checked_mul(self.layout.block_size as u64)
            .ok_or_else(|| ext4_error("inode table size overflows"))?;

        let record_end = record
            .checked_add(inode_size)
            .ok_or_else(|| ext4_error(format!("inode {inode} offset overflows")))?;

        if record_end > table_bytes {
            return Err(ext4_error(format!(
                "inode {inode} extends past the inode table"
            )));
        }

        let start = table
            .checked_mul(self.layout.block_size as u64)
            .ok_or_else(|| ext4_error(format!("inode table block {table} offset overflows")))?;

        start
            .checked_add(record)
            .ok_or_else(|| ext4_error(format!("inode {inode} offset overflows")))
    }

    /// Read bytes at the provided offset
    pub(super) fn read_exact_at(&self, offset: u64, buf: &mut [u8]) -> AccessorResult<()> {
        if buf.is_empty() {
            return Ok(());
        }

        let end = offset.checked_add(buf.len() as u64).ok_or_else(|| {
            ext4_error(format!("read at {offset} length {} overflows", buf.len()))
        })?;
        let filesystem_size = self.filesystem_size()?;

        if end > filesystem_size {
            return Err(ext4_error(format!(
                "read at {offset} length {} exceeds the filesystem",
                buf.len()
            )));
        }

        self.with_reader(|reader| {
            reader.seek(SeekFrom::Start(offset))?;
            reader.read_exact(buf)?;

            Ok(())
        })
    }

    /// Read a single ext4 block
    pub(super) fn read_block(&self, block: u64) -> AccessorResult<Vec<u8>> {
        self.read_blocks(block, 1)
    }

    /// Read multiple continguous filesystem blocks
    pub(super) fn read_blocks(&self, first: u64, count: u64) -> AccessorResult<Vec<u8>> {
        if count == 0 {
            return Ok(Vec::new());
        }

        let end = first
            .checked_add(count)
            .ok_or_else(|| ext4_error(format!("read of {count} blocks at {first} overflows")))?;

        if end > self.layout.block_count {
            return Err(ext4_error(format!(
                "blocks {first}..{end} are outside the filesystem"
            )));
        }

        // Size of the blocks in bytes
        let bytes = count
            .checked_mul(u64::from(self.layout.block_size))
            .ok_or_else(|| ext4_error(format!("read of {count} blocks overflows")))?;

        if bytes > Self::MAX_READ_BYTES {
            return Err(ext4_error(format!(
                "read of {bytes} bytes exceeds the {} byte limit",
                Self::MAX_READ_BYTES
            )));
        }

        let offset = first
            .checked_mul(u64::from(self.layout.block_size))
            .ok_or_else(|| ext4_error(format!("block {first} offset overflows")))?;

        let mut buf = vec![0_u8; bytes as usize];
        self.read_exact_at(offset, &mut buf)?;

        Ok(buf)
    }

    /// Return the parsed `GroupDescriptor`
    pub(super) fn group_descriptors(&self, group: u64) -> AccessorResult<&GroupDescriptor> {
        let index = usize::try_from(group).unwrap_or(usize::MAX);
        self.groups.get(index).ok_or_else(|| {
            ext4_error(format!(
                "block group {group} is outside 0..{}",
                self.groups.len()
            ))
        })
    }

    /// Return the ext4 filesystem target
    pub(super) fn target_path(&self) -> &str {
        &self.target_path
    }

    /// Return the `SuperBlock` structure
    pub(super) fn superblock(&self) -> &SuperBlock {
        &self.superblock
    }

    /// Return the `Ext4Layout` structure
    pub(super) fn layout(&self) -> &Ext4Layout {
        &self.layout
    }

    /// Return the checksum seed value
    pub(super) fn checksum_seed(&self) -> Option<u32> {
        self.checksum_seed
    }

    /// Read one inode value
    ///
    /// Currently only Linux created ext4 filesystems are supported.
    /// GNU Hurd and other types are not supported yet:
    ///
    /// <https://docs.kernel.org/filesystems/ext4/inodes.html#i-osd1>
    pub(super) fn read_inode(&self, inode: u64) -> AccessorResult<Inode> {
        if self.superblock.creator_os != 0 {
            return Err(ext4_error(format!(
                "inode {inode} requires the Linux creator OS, found {}",
                self.superblock.creator()
            )));
        }

        let offset = self.inode_offset(inode)?;
        let mut data = vec![0_u8; self.layout.inode_size as usize];
        self.read_exact_at(offset, &mut data)?;

        let value = parse_inode(&data, self.layout.inode_size)?;
        let inode_number = u32::try_from(inode)
            .map_err(|_| ext4_error(format!("inode {inode} does not fit in 32 bits")))?;

        verify_inode(
            &self.superblock,
            self.checksum_seed,
            inode_number,
            &data,
            &value,
        )?;

        Ok(value)
    }

    /// Return an `Ext4EntryRef` for an inode
    pub(super) fn entry_ref(&self, inode: u64, value: &Inode) -> AccessorResult<Ext4EntryRef> {
        self.inode_offset(inode)?;

        Ok(Ext4EntryRef {
            inode,
            generation: value.generation,
        })
    }

    /// Return the ext4 filesystem reader
    pub(super) fn with_reader<F, T>(&self, operation: F) -> AccessorResult<T>
    where
        F: FnOnce(&mut R) -> AccessorResult<T>,
    {
        let mut reader = self.lock_reader()?;
        operation(&mut reader)
    }

    /// Size of the filesystem in bytes
    fn filesystem_size(&self) -> AccessorResult<u64> {
        self.layout
            .block_count
            .checked_mul(self.layout.block_size as u64)
            .ok_or_else(|| ext4_error("filesystem size overflows"))
    }

    /// Parse the primary `GroupDescriptor` table
    ///
    /// And then parse each `GroupDescriptor` for all block groups
    fn read_group_descriptors(&self) -> AccessorResult<Vec<GroupDescriptor>> {
        let size = self.layout.descriptor_size as u64;
        // Total bytes for the `GroupDescriptor` table
        let byte_len = self
            .layout
            .group_count
            .checked_mul(size)
            .ok_or_else(|| ext4_error("group descriptor table size overflows"))?;

        if byte_len == 0 {
            return Err(ext4_error("group descriptor table is empty"));
        }

        if byte_len > Self::MAX_READ_BYTES {
            return Err(ext4_error(format!(
                "group descriptor table is {byte_len} bytes and exceeds the {} byte limit",
                Self::MAX_READ_BYTES
            )));
        }

        let offset = self
            .layout
            .descriptor_table_block
            .checked_mul(self.layout.block_size as u64)
            .ok_or_else(|| ext4_error("group descriptor table offset overflows"))?;

        let mut table = vec![0_u8; byte_len as usize];
        // Read the `GroupDescriptor` table
        self.read_exact_at(offset, &mut table)?;

        let mut groups = Vec::new();
        let mut group = 0_u64;

        // Get all of the `GroupDescriptor` values
        while group < self.layout.group_count {
            let start = group
                .checked_mul(size)
                .ok_or_else(|| ext4_error(format!("group {group} descriptor offset overflows")))?;

            let start = usize::try_from(start).map_err(|_| {
                ext4_error(format!(
                    "group {group} descriptor offset does not fit in memory"
                ))
            })?;

            let end = start + self.layout.descriptor_size as usize;
            let bytes = &table[start..end];
            let descriptor = parse_group_descriptor(bytes, self.layout.descriptor_size)?;

            let group_number = u32::try_from(group)
                .map_err(|_| ext4_error(format!("block group {group} does not fit in 32 bits")))?;

            verify_group_descriptor(
                &self.superblock,
                self.checksum_seed,
                group_number,
                bytes,
                &descriptor,
            )?;

            validate_group_descriptor(&self.layout, &self.superblock, group, &descriptor)?;
            groups.push(descriptor);
            group += 1;
        }

        Ok(groups)
    }

    /// Ensure our `EXT4` reader is properly locked. Should always be safe since artemis will always be single-threaded
    fn lock_reader(&self) -> AccessorResult<MutexGuard<'_, R>> {
        self.reader.lock().map_err(|err| {
            ext4_path_error(
                &self.target_path(),
                format!("ext4 volume reader lock poisoned: {err:?}"),
            )
        })
    }
}

/// Check for unsupported ext4 flags
fn reject_unsupported(superblock: &SuperBlock) -> AccessorResult<()> {
    if superblock.revision < 1 || !superblock.has_incompatible(SuperBlock::EXTENTS) {
        return Err(ext4_error("extent-based EXT4 is required"));
    }

    let unknown = superblock.unknown_incompatible();
    if unknown != 0 {
        return Err(ext4_error(format!(
            "unsupported incompatible features {unknown:#x}"
        )));
    }

    // Current unsupported flags
    /*
     * <https://docs.kernel.org/filesystems/ext4/super.html>
     * Meta block groups (META_BG) <https://origin.kernel.org/doc/html/latest/filesystems/ext4/blockgroup.html#meta-block-groups>
     * Encryption
     * Inline data <https://docs.kernel.org/filesystems/ext4/inlinedata.html>
     * Extended Attributes Inode <https://origin.kernel.org/doc/html/latest/filesystems/ext4/eainode.html>
     * Case insensitive filenames
     * Compression - Legacy flag
     * Directory data - Never implemented?
     * Journal device - The ext4 journal is a separate attached device (ex: a SSD)
     */
    let incompatible = [
        (SuperBlock::META_BG, "meta_bg"),
        (SuperBlock::ENCRYPT, "encrypt"),
        (SuperBlock::INLINE_DATA, "inline_data"),
        (SuperBlock::EA_INODE, "ea_inode"),
        (SuperBlock::CASEFOLD, "casefold"),
        (SuperBlock::COMPRESSION, "compression"),
        (SuperBlock::DIR_DATA, "dir_data"),
        (SuperBlock::JOURNAL_DEV, "journal_dev"),
    ];

    for (bit, name) in incompatible {
        if superblock.has_incompatible(bit) {
            return Err(ext4_error(format!(
                "unsupported incompatible feature {name}"
            )));
        }
    }

    if superblock.has_read_only(SuperBlock::BIGALLOC) {
        return Err(ext4_error("unsupported read-only feature bigalloc"));
    }

    Ok(())
}

/// Map errors to `AccessorError`
fn ext4_path_error(label: &str, reason: impl Into<String>) -> AccessorError {
    AccessorError::ext4(Some(label.to_string()), reason)
}

#[cfg(test)]
mod tests {
    use super::Ext4Volume;
    use crate::accessor::{
        error::AccessorError,
        filesystem::ext4::format::{
            checksum::crc32c_check,
            group::group_descriptor_crc32c,
            inode::{Inode, inode_crc32c},
            superblock::SuperBlock,
        },
    };
    use common::files::EntryKind;
    use std::{fs::read, io::Cursor, path::PathBuf};

    fn fixture() -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ext4/debian/bookworm/firstblock.raw");
        read(path).unwrap()
    }

    fn image_from(superblock: &[u8]) -> Cursor<Vec<u8>> {
        let mut image = vec![0_u8; 1024];
        image.extend_from_slice(superblock);
        Cursor::new(image)
    }

    fn restore_checksum(superblock: &mut [u8]) {
        let sum = crc32c_check(0xffff_ffff, &superblock[..0x3fc]);
        superblock[0x3fc..0x400].copy_from_slice(&sum.to_le_bytes());
    }

    fn group0() -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ext4/debian/bookworm/descriptor.raw");
        read(path).unwrap()[..64].to_vec()
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

    fn sealed_inode() -> Vec<u8> {
        let mut data = vec![0_u8; 256];
        data[0..2].copy_from_slice(&0x81a4_u16.to_le_bytes());
        data[0x04..0x08].copy_from_slice(&0x100_u32.to_le_bytes());

        data[0x1a..0x1c].copy_from_slice(&1_u16.to_le_bytes());
        data[0x20..0x24].copy_from_slice(&Inode::EXTENTS.to_le_bytes());
        data[0x64..0x68].copy_from_slice(&0xabcd_u32.to_le_bytes());
        data[0x80..0x82].copy_from_slice(&32_u16.to_le_bytes());

        let sum = inode_crc32c(3502656522, 1, &data, 32);
        data[0x7c..0x7e].copy_from_slice(&(sum as u16).to_le_bytes());
        data[0x82..0x84].copy_from_slice(&((sum >> 16) as u16).to_le_bytes());
        data
    }

    fn image_with_inode(inode: &[u8]) -> Cursor<Vec<u8>> {
        let mut descriptor = group0();
        descriptor[8..12].copy_from_slice(&2_u32.to_le_bytes());
        let sum = group_descriptor_crc32c(3502656522, 0, &descriptor);
        descriptor[0x1e..0x20].copy_from_slice(&(sum as u16).to_le_bytes());

        let mut image = vec![0_u8; 8192 + inode.len()];
        let superblock = one_group_superblock();
        image[1024..2048].copy_from_slice(&superblock);
        image[4096..4096 + descriptor.len()].copy_from_slice(&descriptor);
        image[8192..8192 + inode.len()].copy_from_slice(inode);
        Cursor::new(image)
    }

    #[test]
    fn test_open_debian12() {
        let volume = Ext4Volume::open(one_group_image(&group0()), "bookworm").unwrap();

        assert_eq!(volume.target_path(), "bookworm");
        assert_eq!(volume.layout().block_size, 4096);
        assert_eq!(volume.layout().group_count, 1);
        assert_eq!(volume.checksum_seed(), Some(3502656522));
        assert_eq!(volume.groups.len(), 1);

        assert_eq!(volume.group_descriptors(0).unwrap().block_bitmap, 1028);
        assert_eq!(volume.group_descriptors(0).unwrap().inode_bitmap, 1044);
        assert_eq!(volume.group_descriptors(0).unwrap().inode_table, 1060);
        assert_eq!(volume.group_descriptors(0).unwrap().checksum, 61671);

        assert!(volume.superblock().has_incompatible(SuperBlock::RECOVER));
    }

    #[test]
    fn test_open_keeps_the_reader_at_the_filesystem_start() {
        let volume = Ext4Volume::open(one_group_image(&group0()), "bookworm").unwrap();
        let magic = volume
            .with_reader(|reader| {
                use std::io::{Read, Seek, SeekFrom};
                reader.seek(SeekFrom::Start(1024 + 0x38))?;

                let mut magic = [0_u8; 2];

                reader.read_exact(&mut magic)?;
                Ok(u16::from_le_bytes(magic))
            })
            .unwrap();

        assert_eq!(magic, 0xef53);
    }

    #[test]
    fn test_open_rejects_a_short_read() {
        let result = Ext4Volume::open(Cursor::new(vec![0_u8; 64]), "bookworm");
        let err = result.unwrap_err();

        assert!(err.to_string().contains("bookworm"));
        assert!(err.to_string().contains("failed to read the superblock"));
    }

    #[test]
    fn test_open_rejects_bad_checksum() {
        let mut superblock = fixture();
        superblock[0x3fc] ^= 0xff;
        let result = Ext4Volume::open(image_from(&superblock), "bookworm");
        let err = result.unwrap_err();
        assert!(
            matches!(err, AccessorError::Ext4 {  reason,.. } if reason == "superblock checksum mismatch: found 0x4082f809, stored 0x4082f8f6")
        );
    }

    #[test]
    fn test_open_rejects_revision_0() {
        let mut superblock = fixture();
        superblock[0x4c..0x50].copy_from_slice(&0_u32.to_le_bytes());

        let err = Ext4Volume::open(image_from(&superblock), "bookworm").unwrap_err();
        assert!(
            matches!(err, AccessorError::Ext4 { reason,.. } if reason == "extent-based EXT4 is required")
        );
    }

    #[test]
    fn test_open_rejects_missing_extents() {
        let mut superblock = fixture();
        let features =
            u32::from_le_bytes(superblock[0x60..0x64].try_into().unwrap()) & !SuperBlock::EXTENTS;
        superblock[0x60..0x64].copy_from_slice(&features.to_le_bytes());
        restore_checksum(&mut superblock);

        let err = Ext4Volume::open(image_from(&superblock), "bookworm").unwrap_err();
        assert!(
            matches!(err, AccessorError::Ext4 { reason,.. } if reason == "extent-based EXT4 is required")
        )
    }

    #[test]
    fn test_open_rejects_unknown_incompatible_feature() {
        let mut superblock = fixture();
        let features = u32::from_le_bytes(superblock[0x60..0x64].try_into().unwrap()) | (1 << 31);
        superblock[0x60..0x64].copy_from_slice(&features.to_le_bytes());
        restore_checksum(&mut superblock);

        let err = Ext4Volume::open(image_from(&superblock), "bookworm").unwrap_err();
        assert!(
            matches!(err, AccessorError::Ext4 { reason,.. } if reason == "unsupported incompatible features 0x80000000")
        )
    }

    #[test]
    fn test_open_rejects_meta_bg() {
        let mut superblock = fixture();
        let features =
            u32::from_le_bytes(superblock[0x60..0x64].try_into().unwrap()) | SuperBlock::META_BG;
        superblock[0x60..0x64].copy_from_slice(&features.to_le_bytes());
        restore_checksum(&mut superblock);

        let err = Ext4Volume::open(image_from(&superblock), "bookworm").unwrap_err();
        assert!(
            matches!(err, AccessorError::Ext4 { reason,.. } if reason == "unsupported incompatible feature meta_bg")
        )
    }

    #[test]
    fn test_open_rejects_bigalloc() {
        let mut superblock = fixture();
        let features =
            u32::from_le_bytes(superblock[0x64..0x68].try_into().unwrap()) | SuperBlock::BIGALLOC;
        superblock[0x64..0x68].copy_from_slice(&features.to_le_bytes());
        restore_checksum(&mut superblock);

        let err = Ext4Volume::open(image_from(&superblock), "bookworm").unwrap_err();
        assert!(
            matches!(err, AccessorError::Ext4 { reason,.. } if reason == "unsupported read-only feature bigalloc")
        )
    }

    #[test]
    fn test_open_rejects_descriptor_checksum() {
        let mut descriptor = group0();
        descriptor[0] ^= 0xff;

        let err = Ext4Volume::open(one_group_image(&descriptor), "bookworm").unwrap_err();
        assert!(matches!(
            err,
            AccessorError::Ext4 { reason, .. }
                if reason == "group 0 descriptor checksum mismatch: found 0xe675, stored 0xf0e7"
        ));
    }

    #[test]
    fn test_open_rejects_bitmap_overlapping_superblock() {
        let mut descriptor = group0();
        descriptor[0..4].copy_from_slice(&0_u32.to_le_bytes());
        let sum = group_descriptor_crc32c(3502656522, 0, &descriptor);
        descriptor[0x1e..0x20].copy_from_slice(&(sum as u16).to_le_bytes());

        let err = Ext4Volume::open(one_group_image(&descriptor), "bookworm").unwrap_err();
        assert!(matches!(
            err,
            AccessorError::Ext4 { reason, .. }
                if reason == "block bitmap for group 0 overlaps the superblock"
        ));
    }

    #[test]
    fn test_read_block_rejects_a_block_past_the_filesystem() {
        let volume = Ext4Volume::open(one_group_image(&group0()), "bookworm").unwrap();
        let err = volume.read_block(32768).unwrap_err();

        assert!(matches!(
            err,
            AccessorError::Ext4 { reason, .. }
                if reason == "blocks 32768..32769 are outside the filesystem"
        ));
    }

    #[test]
    fn test_read_exact_at_rejects_a_range_past_the_filesystem() {
        let volume = Ext4Volume::open(one_group_image(&group0()), "bookworm").unwrap();
        let err = volume.read_exact_at(32768 * 4096, &mut [0_u8]).unwrap_err();

        assert!(matches!(
            err,
            AccessorError::Ext4 { reason, .. }
                if reason == "read at 134217728 length 1 exceeds the filesystem"
        ));
    }

    #[test]
    fn test_read_block_reads_the_group_descriptor() {
        let volume = Ext4Volume::open(one_group_image(&group0()), "bookworm").unwrap();
        let block = volume.read_block(1).unwrap();

        assert_eq!(block.len(), 4096);
        assert_eq!(&block[..4], &1028_u32.to_le_bytes());
    }

    #[test]
    fn test_inode_offset_uses_the_group_inode_table() {
        let volume = Ext4Volume::open(one_group_image(&group0()), "bookworm").unwrap();

        assert_eq!(volume.inode_offset(1).unwrap(), 1060 * 4096);
        assert_eq!(volume.inode_offset(2).unwrap(), 1060 * 4096 + 256);
        assert_eq!(volume.inode_offset(8160).unwrap(), 1060 * 4096 + 8159 * 256);
    }

    #[test]
    fn test_read_inode_rejects_numbers_outside_the_filesystem() {
        let volume = Ext4Volume::open(one_group_image(&group0()), "bookworm").unwrap();

        let err = volume.read_inode(0).unwrap_err();
        assert!(matches!(
            err,
            AccessorError::Ext4 { reason, .. } if reason == "inode 0 is outside 1..=8160"
        ));

        let err = volume.read_inode(8161).unwrap_err();
        assert!(matches!(
            err,
            AccessorError::Ext4 { reason, .. } if reason == "inode 8161 is outside 1..=8160"
        ));
    }

    #[test]
    fn test_read_inode_reads_the_record_at_the_table_offset() {
        let volume = Ext4Volume::open(image_with_inode(&sealed_inode()), "bookworm").unwrap();
        let inode = volume.read_inode(1).unwrap();

        assert_eq!(inode.kind(), EntryKind::File);
        assert_eq!(inode.size, 0x100);
        assert_eq!(inode.hardlinks, 1);
        assert_eq!(inode.flags, Inode::EXTENTS);
        assert_eq!(inode.generation, 0xabcd);
        assert_eq!(inode.extra_inode_size, 32);

        let reference = volume.entry_ref(1, &inode).unwrap();
        assert_eq!(reference.inode, 1);
        assert_eq!(reference.generation, 0xabcd);
    }

    #[test]
    fn test_read_inode_rejects_a_bad_checksum() {
        let mut inode = sealed_inode();
        inode[0x10] ^= 0xff;

        let volume = Ext4Volume::open(image_with_inode(&inode), "bookworm").unwrap();
        let err = volume.read_inode(1).unwrap_err();
        assert!(matches!(
            err,
            AccessorError::Ext4 { reason, .. } if reason.contains("checksum mismatch")
        ));
    }

    #[test]
    fn test_read_inode_rejects_a_non_linux_creator() {
        let mut superblock = one_group_superblock();
        superblock[0x48..0x4c].copy_from_slice(&1_u32.to_le_bytes());
        restore_checksum(&mut superblock);

        let mut image = vec![0_u8; 8192];
        image[1024..2048].copy_from_slice(&superblock);
        let descriptor = group0();
        image[4096..4096 + descriptor.len()].copy_from_slice(&descriptor);

        let volume = Ext4Volume::open(Cursor::new(image), "bookworm").unwrap();
        let err = volume.read_inode(1).unwrap_err();
        assert!(matches!(
            err,
            AccessorError::Ext4 { reason, .. }
                if reason == "inode 1 requires the Linux creator OS, found Hurd"
        ));
    }
}
