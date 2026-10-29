use common::files::EntryKind;

use crate::{
    accessor::{
        error::AccessorResult,
        filesystem::ext4::format::{
            checksum::crc32c_check,
            superblock::{SuperBlock, ext4_error},
        },
    },
    utils::nom_helper::{nom_take, nom_u16, nom_u32},
};

/// Time to calculate inode timestamp
/// Has nanosecond precision
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct InodeTime {
    /// Seconds since UNIXEPOCH
    pub(crate) seconds: i64,
    /// Nanosecond precision
    pub(crate) nanoseconds: u32,
}

/// Major and minor number from a device block
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DeviceNumber {
    /// Major version number
    pub(crate) major: u32,
    /// Minor version number
    pub(crate) minor: u32,
}

/// Parsed Index Node (`inode`) data
///
/// <https://docs.kernel.org/filesystems/ext4/inodes.html>
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Inode {
    /// Raw mode value
    pub(crate) mode: u16,
    /// User ID value
    pub(crate) uid: u32,
    /// Group ID value
    pub(crate) gid: u32,
    /// Size of file in bytes
    pub(crate) size: u64,
    /// Accessed timestamp
    pub(crate) accessed: InodeTime,
    /// Changed timestamp
    pub(crate) changed: InodeTime,
    /// Modified timestamp
    pub(crate) modified: InodeTime,
    /// Deleted timestamp.
    pub(crate) deleted: InodeTime,
    /// Created timestamp
    pub(crate) created: Option<InodeTime>,
    /// Number of hardlinks to the inode
    pub(crate) hardlinks: u16,
    /// Allocated block count
    pub(crate) blocks: u64,
    /// Inode flags
    pub(crate) flags: u32,
    /// Inode version value
    pub(crate) version: u64,
    /// Raw 60 byte block area
    ///
    /// <https://kernel.org/doc/html/latest/filesystems/ext4/ifork.html#the-contents-of-inode-i-block>
    pub(crate) block_map: [u8; 60],
    /// Inode generation number
    pub(crate) generation: u32,
    /// Block number for external extended attributes
    pub(crate) file_acl: u64,
    /// Number of extra bytes after original 128 byte inode data
    pub(crate) extra_inode_size: u16,
    /// Low 16 bit value for checksum
    pub(crate) checksum_lo: u16,
    /// High 16 bit value for checksum
    pub(crate) checksum_hi: u16,
    /// Inode project ID
    ///
    /// Associated with potential directory quotas
    pub(crate) project_id: Option<u32>,
    /// Raw inline extended attribute bytes
    pub(crate) inline_xattr: Vec<u8>,
}

/// High bit values
struct ExtraFields {
    /// 16 bit high value for checksum
    checksum_hi: u16,
    /// Nanoseconds modified
    modified_extra: Option<u32>,
    /// Nanoseconds accessed
    accessed_extra: Option<u32>,
    /// Nanoseconds changed
    changed_extra: Option<u32>,
    /// Seconds created
    created: Option<u32>,
    /// Nanoseconds created
    created_extra: Option<u32>,
    /// 32 bit high value for version
    version_hi: Option<u32>,
    /// Project ID for inode
    project_id: Option<u32>,
}

impl Inode {
    /// File requires secure deletion (Not used)
    pub(crate) const SECURE_DELETE: u32 = 0x1;
    /// File should be able to be "undeleted" (Not used)
    pub(crate) const UNDO_DELETE: u32 = 0x2;
    /// File is compressed (Not used)
    pub(crate) const COMPRESSED: u32 = 0x4;
    /// File writes must be synchronous
    pub(crate) const SYNC_WRITES: u32 = 0x8;
    /// File is immutable
    pub(crate) const IMMUTABLE: u32 = 0x10;
    /// File can only be appended
    pub(crate) const APPEND: u32 = 0x20;
    /// Dump tools should ignore this file
    pub(crate) const NO_DUMP: u32 = 0x40;
    /// Do not update the access timestamp
    pub(crate) const NO_ACCESS_TIME: u32 = 0x80;
    /// Compression is dirty (Not used)
    pub(crate) const DIRTY_COMPRESSED: u32 = 0x100;
    /// File has compressed clusters (Not used)
    pub(crate) const COMPRESSED_CLUSTERS: u32 = 0x200;
    /// Do not compress this file (Not used)
    pub(crate) const DONT_COMPRESS: u32 = 0x400;
    /// File is encrypted
    pub(crate) const ENCRYPTED: u32 = 0x800;
    /// Directory has hashed indexes
    pub(crate) const DIRECTORY_INDEXES: u32 = 0x1000;
    /// Legacy flag for the "Andrew File System"
    pub(crate) const AFS_MAGIC: u32 = 0x2000;
    /// File data must be written through the journal
    pub(crate) const JOURNAL_DATA: u32 = 0x4000;
    /// File tail should not be merged
    pub(crate) const NO_TAIL: u32 = 0x8000;
    /// All directory data should be written synchronously
    pub(crate) const DIRECTORY_SYNC: u32 = 0x10000;
    /// Top directory hierarchy
    pub(crate) const TOP_DIRECTORY: u32 = 0x20000;
    /// Has `HUGE_FILE` flag. `blocks` uses ext4 blocks instead of 512 byte sectors
    pub(crate) const HUGE_FILE: u32 = 0x40000;
    /// Has `EXTENTS`. Inode uses extents data
    pub(crate) const EXTENTS: u32 = 0x80000;
    /// Verity protected file
    pub(crate) const VERITY: u32 = 0x100000;
    /// Inode stores large extended attributes
    pub(crate) const HAS_INODE_EXTENDED_ATTRIBUTE: u32 = 0x200000;
    /// File has blocks allocated past EOF
    pub(crate) const BLOCKS_EOF: u32 = 0x400000;
    /// Inode is a snapshot
    pub(crate) const SNAPSHOT: u32 = 0x1000000;
    /// Snapshot is being deleted
    pub(crate) const SNAPSHOT_DELETED: u32 = 0x4000000;
    /// Snapshot shrink is completed
    pub(crate) const SNAPSHOT_SHRUNK: u32 = 0x8000000;
    /// Inode has inline data
    pub(crate) const INLINE_DATA: u32 = 0x10000000;
    /// Create children with same project ID
    pub(crate) const PROJECT_INHERIT: u32 = 0x20000000;
    /// Use case insensitive lookups for directory contents
    pub(crate) const CASEFOLD: u32 = 0x40000000;
    /// Reserved flag
    pub(crate) const RESERVED: u32 = 0x80000000;
    /// User visible flags
    pub(crate) const USER_VISIBLE: u32 = 0x705BDFFF;
    /// User modifiable flags
    ///
    /// JOURNAL and EXTENTS flags cannot be changed
    pub(crate) const USER_MODIFIABLE: u32 = 0x604BC0FF;

    /// Return file type based on `mode`
    pub(crate) fn kind(&self) -> EntryKind {
        let mode_type = 0xf000;
        match self.mode & mode_type {
            0x1000 => EntryKind::Pipe,
            0x2000 => EntryKind::CharDevice,
            0x4000 => EntryKind::Directory,
            0x6000 => EntryKind::BlockDevice,
            0x8000 => EntryKind::File,
            0xa000 => EntryKind::Symlink,
            0xc000 => EntryKind::Socket,
            _ => EntryKind::Unsupported,
        }
    }

    /// Return the device number for character or block device
    pub(crate) fn device(&self) -> Option<DeviceNumber> {
        match self.kind() {
            EntryKind::BlockDevice | EntryKind::CharDevice => device_number(&self.block_map).ok(),
            _ => None,
        }
    }

    /// Byte size represented by `blocks`
    pub(crate) fn allocated_bytes(&self, block_size: u32) -> AccessorResult<u64> {
        let unit = if self.flags & Self::HUGE_FILE != 0 {
            block_size as u64
        } else {
            512
        };

        self.blocks.checked_mul(unit).ok_or_else(|| {
            ext4_error(format!(
                "inode allocated size {} * {unit} overflows",
                self.blocks
            ))
        })
    }
}

/// Parse inode data
pub(crate) fn parse_inode(
    data: &[u8],
    inode_size: u16,
    bit64: bool,
    large_dir: bool,
    huge_file: bool,
) -> AccessorResult<Inode> {
    let inode_len = inode_size as usize;
    if data.len() < inode_len {
        return Err(ext4_error(format!(
            "inode is {} bytes, expected {inode_size}",
            data.len()
        )));
    }

    let inode_data = &data[..inode_len];
    let input = inode_data;
    let (input, mode) = nom_u16(input, "inode mode is truncated")?;
    let (input, uid_lo) = nom_u16(input, "inode uid is truncated")?;

    let (input, size_lo) = nom_u32(input, "inode size is truncated")?;
    let (input, atime) = nom_u32(input, "inode access time is truncated")?;
    let (input, ctime) = nom_u32(input, "inode change time is truncated")?;
    let (input, mtime) = nom_u32(input, "inode modification time is truncated")?;
    let (input, dtime) = nom_u32(input, "inode deletion time is truncated")?;

    let (input, gid_lo) = nom_u16(input, "inode gid is truncated")?;
    let (input, hardlinks) = nom_u16(input, "inode link count is truncated")?;

    let (input, blocks_lo) = nom_u32(input, "inode block count is truncated")?;
    let (input, flags) = nom_u32(input, "inode flags are truncated")?;
    let (input, version_lo) = nom_u32(input, "inode version is truncated")?;
    let (input, block) = nom_take(input, 60_u16, "inode block map is truncated")?;

    let (input, generation) = nom_u32(input, "inode generation is truncated")?;
    let (input, file_acl_lo) = nom_u32(input, "inode extended attribute block is truncated")?;
    let (input, size_hi) = nom_u32(input, "inode size high is truncated")?;
    let (input, _obso_faddr) = nom_u32(input, "inode fragment address is truncated")?;

    let (input, blocks_hi) = nom_u16(input, "inode block count high is truncated")?;
    let (input, file_acl_hi) = nom_u16(input, "inode extended attribute block high is truncated")?;
    let (input, uid_hi) = nom_u16(input, "inode uid high is truncated")?;
    let (input, gid_hi) = nom_u16(input, "inode gid high is truncated")?;
    let (input, checksum_lo) = nom_u16(input, "inode checksum is truncated")?;
    let (_input, _reserved) = nom_u16(input, "inode osd2 tail is truncated")?;

    let mut block_map = [0_u8; 60];
    block_map.copy_from_slice(block);

    let mut extra_inode_size = 0_u16;
    let mut checksum_hi = 0_u16;
    let mut ctime_extra = None;
    let mut mtime_extra = None;
    let mut atime_extra = None;

    let mut crtime = None;
    let mut crtime_extra = None;

    let mut version_hi = None;
    let mut project_id = None;
    let mut inline_xattr = Vec::new();

    let inode_min_size = 128;
    if inode_size > inode_min_size {
        let tail = &inode_data[inode_min_size as usize..];
        let (remaining, extra_inode_value) = nom_u16(tail, "inode extra size is truncated")?;

        if extra_inode_value % 4 != 0 {
            return Err(ext4_error(format!(
                "inode extra size {extra_inode_value} is not a multiple of 4"
            )));
        }

        if extra_inode_value as usize > tail.len() {
            return Err(ext4_error(format!(
                "inode extra size {extra_inode_value} extends past inode size {inode_size}"
            )));
        }

        extra_inode_size = extra_inode_value;

        // Check if we have additional bytes
        // We can get additional timestamp precision and created timestamp
        if extra_inode_size > 0 {
            let extra_len = extra_inode_size as usize;
            // We nom the first 2 bytes to determine the inode extra size
            let fields = parse_extra_fields(&remaining[..extra_len - 2])?;

            checksum_hi = fields.checksum_hi;
            ctime_extra = fields.changed_extra;
            mtime_extra = fields.modified_extra;
            atime_extra = fields.accessed_extra;

            crtime = fields.created;
            crtime_extra = fields.created_extra;

            version_hi = fields.version_hi;
            project_id = fields.project_id;
            inline_xattr = tail[extra_len..].to_vec();
        }
    }

    let file_type = mode & 0xf000;
    // Handle both 32 bit and 64 bit systems
    // 64 bit systems use upper size bits
    let use_size_high = large_dir || file_type == 0x8000;
    Ok(Inode {
        mode,
        uid: uid_lo as u32 | (uid_hi as u32) << 16,
        gid: gid_lo as u32 | (gid_hi as u32) << 16,
        size: size_lo as u64
            | if use_size_high {
                (size_hi as u64) << 32
            } else {
                0
            },
        accessed: inode_time(atime, atime_extra),
        changed: inode_time(ctime, ctime_extra),
        modified: inode_time(mtime, mtime_extra),
        deleted: inode_time(dtime, None),
        created: crtime.map(|seconds| inode_time(seconds, crtime_extra)),
        hardlinks,
        blocks: blocks_lo as u64
            | if huge_file {
                (blocks_hi as u64) << 32
            } else {
                0
            },
        flags,
        version: version_lo as u64 | (version_hi.unwrap_or(0) as u64) << 32,
        block_map,
        generation,
        file_acl: file_acl_lo as u64 | if bit64 { (file_acl_hi as u64) << 32 } else { 0 },
        extra_inode_size,
        checksum_lo,
        checksum_hi,
        project_id,
        inline_xattr,
    })
}

/// Validate the inode checksum
pub(crate) fn verify_inode(
    superblock: &SuperBlock,
    checksum_seed: Option<u32>,
    inode: u32,
    data: &[u8],
    value: &Inode,
) -> AccessorResult<()> {
    let inode_min_size = 128;
    if data.len() < inode_min_size {
        return Err(ext4_error(format!(
            "inode {inode} is {} bytes, expected at least 128",
            data.len()
        )));
    }

    if !superblock.has_read_only(SuperBlock::METADATA_CSUM) || value.mode == 0 {
        return Ok(());
    }

    let seed = checksum_seed.ok_or_else(|| ext4_error("metadata checksum seed is missing"))?;
    let hi_fits = checksum_hi_fits(value.extra_inode_size);
    if hi_fits && data.len() < 132 {
        return Err(ext4_error(format!(
            "inode {inode} checksum high is outside the inode record"
        )));
    }

    let actual = inode_crc32c(seed, inode, data, value.extra_inode_size);
    let mut provided = value.checksum_lo as u32;

    let found = if hi_fits {
        provided |= (value.checksum_hi as u32) << 16;
        actual
    } else {
        actual & 0xffff
    };
    if found != provided {
        return Err(ext4_error(format!(
            "inode {inode} checksum mismatch: found {found:#x}, stored {provided:#x}"
        )));
    }

    Ok(())
}

/// Calculate inode checksum
pub(crate) fn inode_crc32c(seed: u32, inode: u32, data: &[u8], extra_inode_size: u16) -> u32 {
    let mut crc = crc32c_check(seed, &inode.to_le_bytes());

    let gen_offset = 100;
    crc = crc32c_check(crc, &data[gen_offset..gen_offset + 4]);

    let checksum_lo = 124;
    crc = crc32c_check(crc, &data[..checksum_lo]);
    crc = crc32c_check(crc, &0_u16.to_le_bytes());

    let inode_size = 128;
    crc = crc32c_check(crc, &data[checksum_lo + 2..inode_size]);

    if data.len() == inode_size {
        return crc;
    }

    let checksum_hi = 130;
    crc = crc32c_check(crc, &data[inode_size..checksum_hi]);
    if checksum_hi_fits(extra_inode_size) {
        crc = crc32c_check(crc, &0_u16.to_le_bytes());
        crc = crc32c_check(crc, &data[checksum_hi + 2..]);
    } else {
        crc = crc32c_check(crc, &data[checksum_hi..]);
    }

    crc
}

/// Parse the extra data that might exist for an inode value
fn parse_extra_fields(mut input: &[u8]) -> AccessorResult<ExtraFields> {
    let checksum_hi = if input.len() >= 2 {
        let (rest, value) = nom_u16(input, "inode checksum high is truncated")?;
        input = rest;
        value
    } else {
        0
    };

    let (input, changed_extra) = optional_u32(input, "inode change time extra is truncated")?;
    let (input, modified_extra) =
        optional_u32(input, "inode modification time extra is truncated")?;
    let (input, accessed_extra) = optional_u32(input, "inode access time extra is truncated")?;

    let (input, created) = optional_u32(input, "inode creation time is truncated")?;
    let (input, created_extra) = optional_u32(input, "inode creation time extra is truncated")?;
    let (input, version_hi) = optional_u32(input, "inode version high is truncated")?;
    let (_, project_id) = optional_u32(input, "inode project id is truncated")?;

    Ok(ExtraFields {
        checksum_hi,
        modified_extra,
        accessed_extra,
        changed_extra,
        created,
        created_extra,
        version_hi,
        project_id,
    })
}

/// Construct `InodeTime` value
fn inode_time(seconds: u32, extra: Option<u32>) -> InodeTime {
    let mut time = InodeTime {
        seconds: i64::from(seconds as i32),
        nanoseconds: 0,
    };

    if let Some(value) = extra {
        time.seconds += i64::from(value & 0x3) << 32;
        time.nanoseconds = value >> 2;
    }

    time
}

/// Check for high bit value for checksum
fn checksum_hi_fits(size: u16) -> bool {
    let checksum_hi = 130;
    extra_field_fits(size, checksum_hi, 2)
}

/// Check if we have extra bytes
fn extra_field_fits(extra_inode_size: u16, offset: u16, size: u16) -> bool {
    offset as u32 + size as u32 <= 128 + extra_inode_size as u32
}

/// Check for the device number for block devices
fn device_number(block_map: &[u8; 60]) -> AccessorResult<DeviceNumber> {
    let (input, first) = nom_u32(block_map, "inode first device number is truncated")?;
    if first != 0 {
        Ok(decode_old_device(first))
    } else {
        let (_, second) = nom_u32(input, "inode second device number is truncated")?;
        Ok(decode_new_device(second))
    }
}

/// Get the old device number version
fn decode_old_device(value: u32) -> DeviceNumber {
    DeviceNumber {
        major: (value >> 8) & 0xff,
        minor: value & 0xff,
    }
}

/// Get the new device number version
fn decode_new_device(value: u32) -> DeviceNumber {
    DeviceNumber {
        major: (value & 0xfff00) >> 8,
        minor: (value & 0xff) | ((value >> 12) & 0xfff00),
    }
}

/// Return 4 byte value if available
fn optional_u32<'a>(input: &'a [u8], reason: &str) -> AccessorResult<(&'a [u8], Option<u32>)> {
    if input.len() < 4 {
        return Ok((input, None));
    }

    let (input, value) = nom_u32(input, reason)?;
    Ok((input, Some(value)))
}

#[cfg(test)]
mod tests {
    use super::{Inode, inode_crc32c, parse_inode, verify_inode};
    use crate::accessor::{
        error::AccessorError,
        filesystem::ext4::format::superblock::{SuperBlock, parse_superblock},
    };
    use common::files::EntryKind;
    use std::{fs::read, path::PathBuf};

    fn read_fixture(name: &str) -> Vec<u8> {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ext4/debian/bookworm");
        path.push(name);
        read(path).unwrap()
    }

    fn superblock() -> SuperBlock {
        parse_superblock(&read_fixture("firstblock.raw")).unwrap()
    }

    fn put_u16(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn sample_inode() -> Vec<u8> {
        let mut data = vec![0_u8; 256];
        put_u16(&mut data, 0x00, 0x81a4);
        put_u16(&mut data, 0x02, 1000);
        put_u32(&mut data, 0x04, 0x100);
        put_u32(&mut data, 0x08, 1700000000);
        put_u32(&mut data, 0x0c, 1700000100);
        put_u32(&mut data, 0x10, 1700000200);

        put_u32(&mut data, 0x14, 90);
        put_u16(&mut data, 0x18, 10);
        put_u16(&mut data, 0x1a, 3);
        put_u32(&mut data, 0x1c, 8);
        put_u32(&mut data, 0x20, Inode::EXTENTS);
        put_u32(&mut data, 0x24, 7);

        data[0x28] = 0x11;
        data[0x29] = 0x22;

        put_u32(&mut data, 0x64, 0xabcd);
        put_u32(&mut data, 0x68, 50);
        put_u32(&mut data, 0x6c, 2);
        put_u16(&mut data, 0x74, 0);
        put_u16(&mut data, 0x76, 1);

        put_u16(&mut data, 0x78, 1);
        put_u16(&mut data, 0x7a, 0);
        put_u16(&mut data, 0x7c, 0x2222);
        put_u16(&mut data, 0x80, 32);
        put_u16(&mut data, 0x82, 0x1111);
        put_u32(&mut data, 0x84, 21);

        put_u32(&mut data, 0x88, 36);
        put_u32(&mut data, 0x8c, 492);
        put_u32(&mut data, 0x90, 1600000000);
        put_u32(&mut data, 0x94, 168);
        put_u32(&mut data, 0x98, 9);
        put_u32(&mut data, 0x9c, 99);

        data[160] = 0xab;
        data
    }

    fn inode_with_mode(mode: u16) -> Inode {
        let mut data = vec![0_u8; 128];
        put_u16(&mut data, 0, mode);
        parse_inode(&data, 128, true, true, true).unwrap()
    }

    #[test]
    fn test_parse_inode_reads_base_and_extra_fields() {
        let inode = parse_inode(&sample_inode(), 256, true, false, true).unwrap();

        assert_eq!(inode.mode, 0x81a4);
        assert_eq!(inode.kind(), EntryKind::File);
        assert_eq!(inode.uid, 65536 + 1000);
        assert_eq!(inode.gid, 10);
        assert_eq!(inode.size, (2_u64 << 32) | 0x100);
        assert_eq!(inode.hardlinks, 3);

        assert_eq!(inode.blocks, 8);
        assert_eq!(inode.allocated_bytes(4096).unwrap(), 8 * 512);
        assert_eq!(inode.flags, Inode::EXTENTS);
        assert_eq!(inode.version, (9_u64 << 32) | 7);

        assert_eq!(inode.block_map[0], 0x11);
        assert_eq!(inode.block_map[1], 0x22);
        assert_eq!(inode.generation, 0xabcd);
        assert_eq!(inode.file_acl, (1_u64 << 32) | 50);

        assert_eq!(inode.extra_inode_size, 32);
        assert_eq!(inode.checksum_lo, 0x2222);
        assert_eq!(inode.checksum_hi, 0x1111);
        assert_eq!(inode.project_id, Some(99));
        assert!(inode.device().is_none());

        assert_eq!(inode.accessed.seconds, 1700000000);
        assert_eq!(inode.accessed.nanoseconds, 123);
        assert_eq!(inode.changed.seconds, 1700000100 + (1_i64 << 32));
        assert_eq!(inode.changed.nanoseconds, 5);
        assert_eq!(inode.modified.seconds, 1700000200);
        assert_eq!(inode.modified.nanoseconds, 9);
        assert_eq!(inode.deleted.seconds, 90);

        assert_eq!(inode.deleted.nanoseconds, 0);
        let created = inode.created.unwrap();

        assert_eq!(created.seconds, 1600000000);
        assert_eq!(created.nanoseconds, 42);

        assert_eq!(inode.inline_xattr.len(), 96);
        assert_eq!(inode.inline_xattr[0], 0xab);
    }

    #[test]
    fn test_kind_comes_from_the_file_type_bits() {
        let cases = [
            (0x8000, EntryKind::File),
            (0x4000, EntryKind::Directory),
            (0xa000, EntryKind::Symlink),
            (0xc000, EntryKind::Socket),
            (0x6000, EntryKind::BlockDevice),
            (0x2000, EntryKind::CharDevice),
            (0x1000, EntryKind::Pipe),
            (0x0000, EntryKind::Unsupported),
        ];

        for (mode, kind) in cases {
            assert_eq!(inode_with_mode(mode).kind(), kind);
        }
    }

    #[test]
    fn test_device_number_uses_old_and_new_encodings() {
        let mut old = vec![0_u8; 128];
        put_u16(&mut old, 0, 0x6000);
        put_u32(&mut old, 0x28, 0x801);

        let device = parse_inode(&old, 128, true, true, true)
            .unwrap()
            .device()
            .unwrap();
        assert_eq!(device.major, 8);
        assert_eq!(device.minor, 1);

        let mut new = vec![0_u8; 128];
        put_u16(&mut new, 0, 0x2000);
        put_u32(&mut new, 0x2c, 0x100800);

        let device = parse_inode(&new, 128, true, true, true)
            .unwrap()
            .device()
            .unwrap();
        assert_eq!(device.major, 8);
        assert_eq!(device.minor, 256);
    }

    #[test]
    fn test_huge_file_counts_filesystem_blocks() {
        let mut inode = parse_inode(&sample_inode(), 256, true, true, true).unwrap();
        inode.flags |= Inode::HUGE_FILE;
        assert_eq!(inode.allocated_bytes(4096).unwrap(), 8 * 4096);
    }

    #[test]
    fn test_parse_inode_rejects_a_short_record() {
        let reason = match parse_inode(&[0_u8; 16], 256, true, true, true) {
            Err(AccessorError::Ext4 { reason, .. }) => reason,
            other => panic!("expected an ext4 error, got {other:?}"),
        };
        assert!(reason.contains("expected 256"));
    }

    #[test]
    fn test_parse_inode_rejects_an_unaligned_extra_size() {
        let mut data = vec![0_u8; 256];
        put_u16(&mut data, 0x80, 2);
        let reason = match parse_inode(&data, 256, true, true, true) {
            Err(AccessorError::Ext4 { reason, .. }) => reason,
            other => panic!("expected an ext4 error, got {other:?}"),
        };
        assert!(reason.contains("not a multiple of 4"));
    }

    #[test]
    fn test_parse_inode_rejects_an_extra_size_past_the_record() {
        let mut data = vec![0_u8; 256];
        put_u16(&mut data, 0x80, 132);
        let reason = match parse_inode(&data, 256, true, true, true) {
            Err(AccessorError::Ext4 { reason, .. }) => reason,
            other => panic!("expected an ext4 error, got {other:?}"),
        };
        assert!(reason.contains("extends past inode size"));
    }

    #[test]
    fn test_verify_inode_checks_metadata_checksum() {
        let superblock = superblock();
        let seed = superblock.checksum_seed();
        let mut data = sample_inode();
        let sum = inode_crc32c(seed.unwrap(), 11, &data, 32);

        put_u16(&mut data, 0x7c, sum as u16);
        put_u16(&mut data, 0x82, (sum >> 16) as u16);

        let parsed = parse_inode(&data, 256, true, true, true).unwrap();
        verify_inode(&superblock, seed, 11, &data, &parsed).unwrap();

        data[0x10] ^= 0xff;
        let parsed = parse_inode(&data, 256, true, true, true).unwrap();
        let err = verify_inode(&superblock, seed, 11, &data, &parsed).unwrap_err();
        assert!(matches!(
            err,
            AccessorError::Ext4 { reason, .. } if reason.contains("checksum mismatch")
        ));
    }

    #[test]
    fn test_verify_inode_compares_16_bits_for_a_128_byte_inode() {
        let superblock = superblock();
        let seed = superblock.checksum_seed().unwrap();
        let mut data = vec![0_u8; 128];

        put_u16(&mut data, 0, 0x8000);
        let sum = inode_crc32c(seed, 2, &data, 0);
        assert_ne!(sum >> 16, 0);
        put_u16(&mut data, 0x7c, sum as u16);

        let parsed = parse_inode(&data, 128, true, true, true).unwrap();
        assert!(parsed.created.is_none());
        assert!(parsed.inline_xattr.is_empty());
        verify_inode(&superblock, Some(seed), 2, &data, &parsed).unwrap();
    }

    #[test]
    fn test_verify_inode_skips_an_unused_record() {
        let superblock = superblock();
        let data = vec![0_u8; 256];
        let sum = inode_crc32c(superblock.checksum_seed().unwrap(), 1, &data, 0);
        assert_ne!(sum & 0xffff, 0);

        let parsed = parse_inode(&data, 256, false, true, true).unwrap();
        assert!(parsed.created.is_none());
        assert_eq!(parsed.extra_inode_size, 0);
        assert!(parsed.inline_xattr.is_empty());

        verify_inode(&superblock, superblock.checksum_seed(), 1, &data, &parsed).unwrap();
    }

    #[test]
    fn test_verify_inode_skips_checksums_when_metadata_csum_is_off() {
        let mut superblock = superblock();
        superblock.read_only_features &= !SuperBlock::METADATA_CSUM;
        let mut data = sample_inode();

        put_u16(&mut data, 0x7c, 0);
        put_u16(&mut data, 0x82, 0);
        let parsed = parse_inode(&data, 256, true, true, true).unwrap();

        verify_inode(&superblock, None, 11, &data, &parsed).unwrap();
    }
}
