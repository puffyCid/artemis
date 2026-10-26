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

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct InodeTime {
    pub(crate) seconds: i64,
    pub(crate) nanoseconds: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct DeviceNumber {
    pub(crate) major: u32,
    pub(crate) minor: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Inode {
    pub(crate) mode: u16,
    pub(crate) uid: u32,
    pub(crate) gid: u32,
    pub(crate) size: u64,
    pub(crate) accessed: InodeTime,
    pub(crate) changed: InodeTime,
    pub(crate) modified: InodeTime,
    pub(crate) deleted: InodeTime,
    pub(crate) created: Option<InodeTime>,
    pub(crate) hardlinks: u16,
    pub(crate) blocks: u64,
    pub(crate) flags: u32,
    pub(crate) version: u64,
    pub(crate) block_map: [u8; 60],
    pub(crate) generation: u32,
    pub(crate) file_acl: u64,
    pub(crate) extra_inode_size: u16,
    pub(crate) checksum_lo: u16,
    pub(crate) checksum_hi: u16,
    pub(crate) project_id: Option<u32>,
    pub(crate) inline_xattr: Vec<u8>,
}

struct ExtraFields {
    checksum_hi: u16,
    modified_extra: Option<u32>,
    accessed_extra: Option<u32>,
    changed_extra: Option<u32>,
    created: Option<u32>,
    created_extra: Option<u32>,
    version_hi: Option<u32>,
    project_id: Option<u32>,
}

impl Inode {
    pub(crate) const HUGE_FILE: u32 = 0x40000;
    pub(crate) const EXTENTS: u32 = 0x80000;

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

pub(crate) fn parse_inode(data: &[u8], inode_size: u16) -> AccessorResult<Inode> {
    let inode_len = inode_size as usize;
    if data.len() < inode_len {
        return Err(ext4_error(format!(
            "inode is {} bytes, expected {inode_size}",
            data.len()
        )));
    }

    let input = &data[..inode_len];
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
        let tail = &data[inode_min_size as usize..];
        let (_rest, extra_inode_value) = nom_u16(tail, "inode extra size is truncated")?;

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
        if extra_inode_size == 0 {
            inline_xattr = tail.to_vec();
        } else {
            let extra_len = extra_inode_size as usize;
            let fields = parse_extra_fields(&tail[2..extra_len])?;

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

    Ok(Inode {
        mode,
        uid: uid_lo as u32 | (uid_hi as u32) << 16,
        gid: gid_lo as u32 | (gid_hi as u32) << 16,
        size: size_lo as u64 | (size_hi as u64) << 32,
        accessed: inode_time(atime, atime_extra),
        changed: inode_time(ctime, ctime_extra),
        modified: inode_time(mtime, mtime_extra),
        deleted: inode_time(dtime, None),
        created: crtime.map(|seconds| inode_time(seconds, crtime_extra)),
        hardlinks,
        blocks: blocks_lo as u64 | (blocks_hi as u64) << 32,
        flags,
        version: version_lo as u64 | (version_hi.unwrap_or(0) as u64) << 32,
        block_map,
        generation,
        file_acl: file_acl_lo as u64 | (file_acl_hi as u64) << 32,
        extra_inode_size,
        checksum_lo,
        checksum_hi,
        project_id,
        inline_xattr,
    })
}

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

pub(crate) fn inode_crc32c(seed: u32, inode: u32, data: &[u8], extra_inode_size: u16) -> u32 {
    let mut crc = crc32c_check(seed, &inode.to_le_bytes());

    crc = crc32c_check(crc, &data[100..104]);
    crc = crc32c_check(crc, &data[..124]);
    crc = crc32c_check(crc, &0_u16.to_le_bytes());
    crc = crc32c_check(crc, &data[126..128]);

    if data.len() == 128 {
        return crc;
    }

    crc = crc32c_check(crc, &data[128..130]);
    if checksum_hi_fits(extra_inode_size) {
        crc = crc32c_check(crc, &0_u16.to_le_bytes());
        crc = crc32c_check(crc, &data[132..]);
    } else {
        crc = crc32c_check(crc, &data[130..]);
    }

    crc
}

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

fn inode_time(seconds: u32, extra: Option<u32>) -> InodeTime {
    let mut time = InodeTime {
        seconds: seconds as i64,
        nanoseconds: 0,
    };

    if let Some(value) = extra {
        time.seconds += (value as i64 & 0x3) << 32;
        time.nanoseconds = value >> 2;
    }

    time
}

fn checksum_hi_fits(size: u16) -> bool {
    extra_field_fits(size, 128, 2)
}

fn extra_field_fits(extra_inode_size: u16, offset: u16, size: u16) -> bool {
    offset as u32 + size as u32 <= 128 + extra_inode_size as u32
}

fn device_number(block_map: &[u8; 60]) -> AccessorResult<DeviceNumber> {
    let (input, first) = nom_u32(block_map, "inode first device number is truncated")?;
    if first != 0 {
        Ok(decode_old_device(first))
    } else {
        let (_, second) = nom_u32(input, "inode second device number is truncated")?;
        Ok(decode_new_device(second))
    }
}

fn decode_old_device(value: u32) -> DeviceNumber {
    DeviceNumber {
        major: (value >> 8) & 0xff,
        minor: value & 0xff,
    }
}

fn decode_new_device(value: u32) -> DeviceNumber {
    DeviceNumber {
        major: (value & 0xfff00) >> 8,
        minor: (value & 0xff) | ((value >> 12) & 0xfff00),
    }
}

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
        parse_inode(&data, 128).unwrap()
    }

    #[test]
    fn test_parse_inode_reads_base_and_extra_fields() {
        let inode = parse_inode(&sample_inode(), 256).unwrap();

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

        let device = parse_inode(&old, 128).unwrap().device().unwrap();
        assert_eq!(device.major, 8);
        assert_eq!(device.minor, 1);

        let mut new = vec![0_u8; 128];
        put_u16(&mut new, 0, 0x2000);
        put_u32(&mut new, 0x2c, 0x100800);
        let device = parse_inode(&new, 128).unwrap().device().unwrap();
        assert_eq!(device.major, 8);
        assert_eq!(device.minor, 256);
    }

    #[test]
    fn test_huge_file_counts_filesystem_blocks() {
        let mut inode = parse_inode(&sample_inode(), 256).unwrap();
        inode.flags |= Inode::HUGE_FILE;
        assert_eq!(inode.allocated_bytes(4096).unwrap(), 8 * 4096);
    }

    #[test]
    fn test_parse_inode_rejects_a_short_record() {
        let reason = match parse_inode(&[0_u8; 16], 256) {
            Err(AccessorError::Ext4 { reason, .. }) => reason,
            other => panic!("expected an ext4 error, got {other:?}"),
        };
        assert!(reason.contains("expected 256"));
    }

    #[test]
    fn test_parse_inode_rejects_an_unaligned_extra_size() {
        let mut data = vec![0_u8; 256];
        put_u16(&mut data, 0x80, 2);
        let reason = match parse_inode(&data, 256) {
            Err(AccessorError::Ext4 { reason, .. }) => reason,
            other => panic!("expected an ext4 error, got {other:?}"),
        };
        assert!(reason.contains("not a multiple of 4"));
    }

    #[test]
    fn test_parse_inode_rejects_an_extra_size_past_the_record() {
        let mut data = vec![0_u8; 256];
        put_u16(&mut data, 0x80, 132);
        let reason = match parse_inode(&data, 256) {
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

        let parsed = parse_inode(&data, 256).unwrap();
        verify_inode(&superblock, seed, 11, &data, &parsed).unwrap();

        data[0x10] ^= 0xff;
        let parsed = parse_inode(&data, 256).unwrap();
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

        let parsed = parse_inode(&data, 128).unwrap();
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

        let parsed = parse_inode(&data, 256).unwrap();
        assert_eq!(parsed.extra_inode_size, 0);
        assert_eq!(parsed.inline_xattr.len(), 128);
        assert!(parsed.created.is_none());
        verify_inode(&superblock, superblock.checksum_seed(), 1, &data, &parsed).unwrap();
    }

    #[test]
    fn test_verify_inode_skips_checksums_when_metadata_csum_is_off() {
        let mut superblock = superblock();
        superblock.read_only_features &= !SuperBlock::METADATA_CSUM;
        let mut data = sample_inode();

        put_u16(&mut data, 0x7c, 0);
        put_u16(&mut data, 0x82, 0);
        let parsed = parse_inode(&data, 256).unwrap();

        verify_inode(&superblock, None, 11, &data, &parsed).unwrap();
    }
}
