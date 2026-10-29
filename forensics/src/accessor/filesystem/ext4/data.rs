use crate::accessor::{
    error::AccessorResult,
    filesystem::ext4::{
        extent::{ExtentRun, map_extent},
        format::{inode::Inode, superblock::ext4_error},
        volume::Ext4Volume,
    },
};
use common::files::EntryKind;
use std::io::{Read, Seek};

/// Read bytes associated with `Extent` data. Typically files, directories, or symlinks
///
/// Should not read past the inode size value
pub(crate) fn read_file_at<R>(
    volume: &Ext4Volume<R>,
    inode_number: u64,
    inode: &Inode,
    offset: u64,
    buf: &mut [u8],
) -> AccessorResult<usize>
where
    R: Read + Seek + Send,
{
    // Encrypted data not supported yet
    if inode.flags & Inode::ENCRYPTED != 0 {
        return Err(ext4_error(format!("inode {inode_number} is encrypted")));
    }

    // Only Files, directories, or symlink data can be read currently
    // We do not follow symlink
    if !matches!(
        inode.kind(),
        EntryKind::File | EntryKind::Directory | EntryKind::Symlink
    ) {
        return Err(ext4_error(format!("inode {inode_number} has no file data")));
    }

    if buf.is_empty() || offset >= inode.size {
        return Ok(0);
    }

    let block_size = volume.layout().block_size as u64;
    let available = usize::try_from(inode.size - offset).unwrap_or(usize::MAX);

    // How many bytes to read
    let want = available.min(buf.len());
    let mut bytes = 0_usize;

    while bytes < want {
        let filled_bytes = u64::try_from(bytes)
            .map_err(|err| ext4_error(format!("file read length does not fit: {err:?}")))?;
        let file_offset = offset
            .checked_add(filled_bytes)
            .ok_or_else(|| ext4_error(format!("file offset {offset} length {bytes} overflows")))?;

        // Determine logical block number
        // Ex: 4100 (offset) / 4096 = 1 (logical block 1)
        let logical = file_offset / block_size;
        let logical_block = u32::try_from(logical).map_err(|err| {
            ext4_error(format!(
                "inode {inode_number} logical block {logical} does not fit in 32 bits: {err:?}"
            ))
        })?;

        // Attempt to get the extent leaf for the data
        let run = map_extent(volume, inode_number, inode, logical_block)?;
        let blocks = match run {
            ExtentRun::Hole { length, .. } => length,
            ExtentRun::Data { length, .. } | ExtentRun::Unwritten { length, .. } => length as u64,
        };

        if blocks == 0 {
            return Err(ext4_error(format!(
                "extent run at logical block {logical} is empty"
            )));
        }

        let run_bytes = blocks
            .checked_mul(block_size)
            .ok_or_else(|| ext4_error("extent run size overflows"))?;

        // Determine offset within the logical block
        // Ex: 4100 (offset) % 4096 = 4 (offset 4 inside logical block 1)
        let within = file_offset % block_size;
        let in_run = run_bytes.checked_sub(within).ok_or_else(|| {
            ext4_error(format!(
                "file offset {file_offset} is outside its extent run"
            ))
        })?;

        let remaining = u64::try_from(want - bytes)
            .map_err(|err| ext4_error(format!("file read length does not fit: {err:?}")))?;
        let take = usize::try_from(in_run.min(remaining)).map_err(|err| {
            ext4_error(format!("file read length does not fit in memory: {err:?}"))
        })?;

        if take == 0 {
            return Err(ext4_error(format!(
                "extent run at logical block {logical} is empty"
            )));
        }

        match run {
            // If unallocated, fill with 0 (common with sparse files)
            ExtentRun::Hole { .. } | ExtentRun::Unwritten { .. } => {
                buf[bytes..bytes + take].fill(0);
            }
            ExtentRun::Data { physical, .. } => {
                let disk = physical
                    .checked_mul(block_size)
                    .and_then(|base| base.checked_add(within))
                    .ok_or_else(|| ext4_error("extent byte offset overflows"))?;

                volume.read_exact_at(disk, &mut buf[bytes..bytes + take])?;
            }
        }

        bytes += take;
    }

    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::read_file_at;
    use crate::accessor::{
        error::{AccessorError, AccessorResult},
        filesystem::ext4::{
            format::{
                checksum::crc32c_check,
                inode::{Inode, parse_inode},
            },
            volume::Ext4Volume,
        },
    };
    use std::{fs::read, io::Cursor, path::PathBuf};

    const BLOCK: usize = 4096;

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

    fn plant(blocks: &[(u64, &[u8])]) -> Ext4Volume<Cursor<Vec<u8>>> {
        let mut image = one_group_image(&group0()).into_inner();

        for (block, contents) in blocks {
            let offset = usize::try_from(*block).unwrap() * BLOCK;
            let end = offset + contents.len();
            if image.len() < end {
                image.resize(end, 0);
            }
            image[offset..end].copy_from_slice(contents);
        }

        let volume = Ext4Volume::open(Cursor::new(image), "bookworm").unwrap();
        assert_eq!(volume.layout().block_size, BLOCK as u32);
        volume
    }

    fn open_volume() -> Ext4Volume<Cursor<Vec<u8>>> {
        plant(&[])
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

    fn inode_with_tree(tree: &[u8]) -> Inode {
        assert!(tree.len() <= 60);
        let mut data = vec![0_u8; 256];
        data[0..2].copy_from_slice(&0x8000_u16.to_le_bytes());
        data[0x20..0x24].copy_from_slice(&Inode::EXTENTS.to_le_bytes());

        data[0x28..0x28 + tree.len()].copy_from_slice(tree);
        data[0x64..0x68].copy_from_slice(&0xabcd_u32.to_le_bytes());
        parse_inode(&data, 256, true, true, true).unwrap()
    }

    fn file_inode(tree: &[u8], size: u64) -> Inode {
        let mut inode = inode_with_tree(tree);
        inode.size = size;
        inode
    }

    fn leaf_root(logical: u32, len: u16, physical: u64) -> Vec<u8> {
        let mut data = vec![0_u8; 60];
        write_header(&mut data, 1, 4, 0);
        write_leaf(&mut data, 12, logical, len, physical);
        data
    }

    fn empty_leaf() -> Vec<u8> {
        let mut data = vec![0_u8; 60];
        write_header(&mut data, 0, 4, 0);
        data
    }

    fn ext4_reason(result: Result<impl std::fmt::Debug, AccessorError>) -> String {
        match result {
            Err(AccessorError::Ext4 { reason, .. }) => reason,
            other => panic!("expected an ext4 error, got {other:?}"),
        }
    }

    fn read_bytes(
        volume: &Ext4Volume<Cursor<Vec<u8>>>,
        inode: &Inode,
        offset: u64,
        len: usize,
    ) -> AccessorResult<Vec<u8>> {
        let mut buf = vec![0xff_u8; len];
        let count = read_file_at(volume, 1, inode, offset, &mut buf)?;
        buf.truncate(count);
        Ok(buf)
    }

    #[test]
    fn test_read_file_at_reads_the_middle_of_a_block() {
        let mut block = vec![0_u8; BLOCK];
        block[100..105].copy_from_slice(b"abcde");
        let volume = plant(&[(50, &block)]);
        let inode = file_inode(&leaf_root(0, 1, 50), 200);

        assert_eq!(read_bytes(&volume, &inode, 100, 5).unwrap(), b"abcde");
        assert_eq!(read_bytes(&volume, &inode, 100, 3).unwrap(), b"abc");
    }

    #[test]
    fn test_read_file_at_stops_at_the_file_size() {
        let mut block = vec![0_u8; BLOCK];
        block[..16].copy_from_slice(b"hello\nSLACK!!!!!");
        let volume = plant(&[(50, &block)]);
        let inode = file_inode(&leaf_root(0, 1, 50), 6);

        let mut buf = vec![0xff_u8; 16];
        let count = read_file_at(&volume, 1, &inode, 0, &mut buf).unwrap();

        assert_eq!(count, 6);
        assert_eq!(&buf[..6], b"hello\n");
        assert_eq!(&buf[6..], &[0xff_u8; 10]);

        assert_eq!(read_bytes(&volume, &inode, 4, 10).unwrap(), b"o\n");
    }

    #[test]
    fn test_read_file_at_returns_zero_at_the_end_of_the_file() {
        let volume = open_volume();
        let inode = file_inode(&leaf_root(0, 1, 50), 6);
        let mut buf = vec![0xff_u8; 4];

        assert_eq!(read_file_at(&volume, 1, &inode, 6, &mut buf).unwrap(), 0);
        assert_eq!(read_file_at(&volume, 1, &inode, 7, &mut buf).unwrap(), 0);
        assert_eq!(buf, vec![0xff_u8; 4]);

        let mut empty = [];
        assert_eq!(read_file_at(&volume, 1, &inode, 0, &mut empty).unwrap(), 0);
    }

    #[test]
    fn test_read_file_at_zero_fills_a_hole_and_then_reads_data() {
        let mut block = vec![0_u8; BLOCK];
        block[..6].copy_from_slice(b"abcdef");
        let volume = plant(&[(50, &block)]);
        let inode = file_inode(&leaf_root(1, 1, 50), BLOCK as u64 + 6);

        assert_eq!(read_bytes(&volume, &inode, 0, 4).unwrap(), [0, 0, 0, 0]);
        assert_eq!(
            read_bytes(&volume, &inode, BLOCK as u64, 10).unwrap(),
            b"abcdef"
        );
    }

    #[test]
    fn test_read_file_at_zero_fills_an_unwritten_extent() {
        let block = vec![0xff_u8; BLOCK];
        let volume = plant(&[(80, &block)]);
        let inode = file_inode(&leaf_root(0, 0x8001, 80), 8);

        assert_eq!(read_bytes(&volume, &inode, 0, 8).unwrap(), [0_u8; 8]);
    }

    #[test]
    fn test_read_file_at_clips_a_hole_to_the_buffer_and_file_size() {
        let volume = open_volume();
        let inode = file_inode(&empty_leaf(), 100);

        assert_eq!(read_bytes(&volume, &inode, 0, 4).unwrap(), [0, 0, 0, 0]);
        assert_eq!(read_bytes(&volume, &inode, 80, 50).unwrap(), vec![0_u8; 20]);
    }

    #[test]
    fn test_read_file_at_spans_initialized_data_into_a_hole() {
        let block = vec![b'A'; BLOCK];
        let volume = plant(&[(50, &block)]);
        let inode = file_inode(&leaf_root(0, 1, 50), BLOCK as u64 + 4);

        assert_eq!(
            read_bytes(&volume, &inode, BLOCK as u64 - 2, 8).unwrap(),
            [b'A', b'A', 0, 0, 0, 0]
        );
    }

    #[test]
    fn test_read_file_at_reads_across_a_block_boundary() {
        let mut first = vec![0_u8; BLOCK];
        let mut second = vec![0_u8; BLOCK];
        first[BLOCK - 6..].fill(b'A');
        second[..14].fill(b'B');
        let volume = plant(&[(50, &first), (51, &second)]);
        let inode = file_inode(&leaf_root(0, 2, 50), 8192);

        let mut expected = vec![b'A'; 6];
        expected.extend(vec![b'B'; 14]);
        assert_eq!(
            read_bytes(&volume, &inode, BLOCK as u64 - 6, 20).unwrap(),
            expected
        );
    }

    #[test]
    fn test_read_file_at_reads_a_directory() {
        let mut block = vec![0_u8; BLOCK];
        block[..4].copy_from_slice(b"dir!");
        let volume = plant(&[(50, &block)]);
        let mut inode = file_inode(&leaf_root(0, 1, 50), 4);
        inode.mode = 0x4000;

        assert_eq!(read_bytes(&volume, &inode, 0, 4).unwrap(), b"dir!");
    }

    #[test]
    fn test_read_file_at_rejects_an_encrypted_inode() {
        let volume = open_volume();
        let mut inode = file_inode(&leaf_root(0, 1, 50), 4);
        inode.flags |= Inode::ENCRYPTED;

        let reason = ext4_reason(read_file_at(&volume, 7, &inode, 0, &mut [0_u8; 4]));
        assert!(reason.contains("encrypted"), "{reason}");
    }

    #[test]
    fn test_read_file_at_rejects_an_inode_with_no_file_data() {
        let volume = open_volume();
        let mut inode = file_inode(&leaf_root(0, 1, 50), 4);

        for mode in [0x1000_u16, 0x2000, 0x6000, 0xc000, 0] {
            inode.mode = mode;
            let reason = ext4_reason(read_file_at(&volume, 1, &inode, 0, &mut [0_u8; 4]));
            assert!(reason.contains("no file data"), "{mode:#x}: {reason}");
        }
    }

    #[test]
    fn test_read_file_at_rejects_a_logical_block_past_32_bits() {
        let volume = open_volume();
        let block_size = 4096_u64;
        let offset = (u64::from(u32::MAX) + 1) * block_size;
        let inode = file_inode(&empty_leaf(), offset + 8);

        let reason = ext4_reason(read_file_at(&volume, 1, &inode, offset, &mut [0_u8; 4]));
        assert!(reason.contains("does not fit in 32 bits"), "{reason}");
    }

    #[test]
    fn test_read_file_at_rejects_a_symlink_without_extents() {
        let volume = open_volume();
        let mut inode = file_inode(&leaf_root(0, 1, 50), 4);
        inode.mode = 0xa000;
        inode.flags &= !Inode::EXTENTS;

        let reason = ext4_reason(read_file_at(&volume, 1, &inode, 0, &mut [0_u8; 4]));
        assert!(reason.contains("does not use extents"), "{reason}");
    }
}
