use crate::accessor::{
    error::{AccessorError, AccessorResult},
    filesystem::ext4::{
        data::read_file_at,
        format::{
            directory::{DirectoryEntry, parse_directory_block, verify_directory_block},
            inode::Inode,
            superblock::{SuperBlock, ext4_error},
        },
        volume::Ext4Volume,
    },
};
use common::files::EntryKind;
use std::io::{Read, Seek};

/// Read and parse `Directory` entries from provided `Inode` number
pub(crate) fn read_directory<R>(
    volume: &Ext4Volume<R>,
    inode_number: u64,
    inode: &Inode,
) -> AccessorResult<Vec<DirectoryEntry>>
where
    R: Read + Seek + Send,
{
    let mut entries = Vec::new();
    visit_directory(volume, inode_number, inode, |entry| {
        if entry.name != b"." && entry.name != b".." {
            entries.push(entry);
        }

        Ok(false)
    })?;

    Ok(entries)
}

/// Find a `Directory` entry by name and return its `Inode` number
pub(crate) fn lookup_name<R>(
    volume: &Ext4Volume<R>,
    inode_number: u64,
    inode: &Inode,
    name: &[u8],
) -> AccessorResult<Option<u32>>
where
    R: Read + Seek + Send,
{
    let mut found = None;
    visit_directory(volume, inode_number, inode, |entry| {
        if entry.name == name {
            found = Some(entry.inode);
            return Ok(true);
        }
        Ok(false)
    })?;

    Ok(found)
}

/// Return the `Inode` number and `Inode` data for a provided path
pub(crate) fn resolve_path<R>(volume: &Ext4Volume<R>, path: &str) -> AccessorResult<(u64, Inode)>
where
    R: Read + Seek + Send,
{
    let root_inode = 2;
    let mut inode_number = root_inode;
    let mut inode = volume.read_inode(root_inode)?;

    for component in path.split('/').filter(|part| !part.is_empty()) {
        if inode.kind() != EntryKind::Directory {
            return Err(AccessorError::not_a_directory(path));
        }

        let Some(next) = lookup_name(volume, inode_number, &inode, component.as_bytes())? else {
            return Err(AccessorError::not_found(path));
        };

        inode_number = next as u64;
        inode = volume.read_inode(inode_number)?;
    }

    Ok((inode_number, inode))
}

/// Read the symbolic path (it does not follow the link!)
/// It just reads the path it points to (not the data)
pub(crate) fn read_symlink<R>(
    volume: &Ext4Volume<R>,
    inode_number: u64,
    inode: &Inode,
) -> AccessorResult<Vec<u8>>
where
    R: Read + Seek + Send,
{
    if inode.flags & Inode::ENCRYPTED != 0 {
        return Err(ext4_error(format!("inode {inode_number} is encrypted")));
    }

    if inode.kind() != EntryKind::Symlink {
        return Err(ext4_error(format!("inode {inode_number} is not a symlink")));
    }

    let max_symlink_size = 64 * 1024;
    if inode.size > max_symlink_size {
        return Err(ext4_error(format!(
            "inode {inode_number} symlink target is {} bytes",
            inode.size
        )));
    }

    let size = usize::try_from(inode.size)
        .map_err(|err| ext4_error(format!("symlink target does not fit in memory: {err:?}")))?;

    if inode.flags & Inode::EXTENTS != 0 {
        let mut buf = vec![0_u8; size];
        let bytes = read_file_at(volume, inode_number, inode, 0, &mut buf)?;
        if bytes != size {
            return Err(ext4_error(format!(
                "inode {inode_number} symlink is short: found {bytes} bytes, expected {size}"
            )));
        }

        return Ok(buf);
    }

    if size > inode.block_map.len() {
        return Err(ext4_error(format!(
            "inode {inode_number} symlink is not stored in the inode"
        )));
    }

    Ok(inode.block_map[..size].to_vec())
}

/// Loop through each `Directory` entry
///
/// `visit` returns true to break the loop
fn visit_directory<R>(
    volume: &Ext4Volume<R>,
    inode_number: u64,
    inode: &Inode,
    mut visit: impl FnMut(DirectoryEntry) -> AccessorResult<bool>,
) -> AccessorResult<()>
where
    R: Read + Seek + Send,
{
    if inode.kind() != EntryKind::Directory {
        return Err(ext4_error(format!(
            "inode {inode_number} is not a directory"
        )));
    }

    let block_size = volume.layout().block_size as u64;
    if block_size == 0 || !inode.size.is_multiple_of(block_size) {
        return Err(ext4_error(format!(
            "inode {inode_number} directory size {} is not a multiple of the block size",
            inode.size
        )));
    }

    let inode_u32 = u32::try_from(inode_number).map_err(|err| {
        ext4_error(format!(
            "inode {inode_number} does not fit in 32 bits: {err:?}"
        ))
    })?;

    let filetype = volume.superblock().has_incompatible(SuperBlock::FILETYPE);
    let buf_size = usize::try_from(block_size).map_err(|err| {
        ext4_error(format!(
            "directory block size does not fit in memory: {err:?}"
        ))
    })?;

    let mut index = 0_u64;
    while index < inode.size / block_size {
        let offset = index * block_size;
        let mut block = vec![0_u8; buf_size];

        let bytes = read_file_at(volume, inode_number, inode, offset, &mut block)?;
        if bytes != block.len() {
            return Err(ext4_error(format!(
                "inode {inode_number} directory block {index} is short"
            )));
        }

        verify_directory_block(
            volume.superblock(),
            volume.checksum_seed(),
            inode_u32,
            inode.generation,
            &block,
        )?;

        for entry in parse_directory_block(&block, volume.layout().block_size, filetype)? {
            if visit(entry)? {
                return Ok(());
            }
        }

        index += 1;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{lookup_name, read_directory, read_symlink, resolve_path};
    use crate::accessor::{
        error::AccessorError,
        filesystem::ext4::{
            format::{
                checksum::crc32c_check,
                directory::parse_directory_block,
                inode::{Inode, parse_inode},
            },
            volume::Ext4Volume,
        },
    };
    use std::{
        fs::{File, read},
        io::Cursor,
        path::PathBuf,
    };

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

    fn directory_inode(tree: &[u8], size: u64) -> Inode {
        let mut data = vec![0_u8; 256];
        data[0..2].copy_from_slice(&0x4000_u16.to_le_bytes());
        data[0x20..0x24].copy_from_slice(&Inode::EXTENTS.to_le_bytes());
        data[0x28..0x28 + tree.len()].copy_from_slice(tree);
        data[0x64..0x68].copy_from_slice(&0xabcd_u32.to_le_bytes());

        let mut inode = parse_inode(&data, 256).unwrap();
        inode.size = size;
        inode
    }

    fn leaf_root(logical: u32, len: u16, physical: u64) -> Vec<u8> {
        let mut data = vec![0_u8; 60];
        write_header(&mut data, 1, 4, 0);
        write_leaf(&mut data, 12, logical, len, physical);
        data
    }

    fn write_entry(
        data: &mut [u8],
        offset: usize,
        inode: u32,
        record_length: u16,
        file_type: u8,
        name: &[u8],
    ) {
        put_u32(data, offset, inode);
        put_u16(data, offset + 4, record_length);
        data[offset + 6] = u8::try_from(name.len()).unwrap();
        data[offset + 7] = file_type;
        data[offset + 8..offset + 8 + name.len()].copy_from_slice(name);
    }

    fn seal_leaf(data: &mut [u8], seed: u32, inode: u32, generation: u32) {
        let tail = data.len() - 12;
        put_u16(data, tail + 4, 12);
        data[tail + 7] = 0xde;

        let mut checksum = crc32c_check(seed, &inode.to_le_bytes());
        checksum = crc32c_check(checksum, &generation.to_le_bytes());
        checksum = crc32c_check(checksum, &data[..tail]);
        put_u32(data, tail + 8, checksum);
    }

    fn ext4_reason(result: Result<impl std::fmt::Debug, AccessorError>) -> String {
        match result {
            Err(AccessorError::Ext4 { reason, .. }) => reason,
            other => panic!("expected an ext4 error, got {other:?}"),
        }
    }

    fn names(
        entries: &[crate::accessor::filesystem::ext4::format::directory::DirectoryEntry],
    ) -> Vec<&[u8]> {
        entries.iter().map(|entry| entry.name.as_slice()).collect()
    }

    #[test]
    fn test_read_directory_omits_dot_entries_and_lookup_keeps_them() {
        let volume = plant(&[]);
        let seed = volume.checksum_seed().unwrap();
        let mut block = vec![0_u8; BLOCK];
        write_entry(&mut block, 0, 12, 12, 2, b".");
        write_entry(&mut block, 12, 2, 12, 2, b"..");
        write_entry(&mut block, 24, 15, BLOCK as u16 - 24 - 12, 1, b"hello");

        seal_leaf(&mut block, seed, 12, 0xabcd);
        let volume = plant(&[(50, &block)]);
        let inode = directory_inode(&leaf_root(0, 1, 50), BLOCK as u64);

        assert_eq!(
            names(&read_directory(&volume, 12, &inode).unwrap()),
            [b"hello".as_slice()]
        );
        assert_eq!(lookup_name(&volume, 12, &inode, b"..").unwrap(), Some(2));
        assert_eq!(
            lookup_name(&volume, 12, &inode, b"hello").unwrap(),
            Some(15)
        );
        assert_eq!(lookup_name(&volume, 12, &inode, b"missing").unwrap(), None);
    }

    #[test]
    fn test_read_symlink_reads_a_fast_target_and_an_extent_target() {
        let volume = plant(&[]);
        let mut fast = directory_inode(&leaf_root(0, 1, 50), 4);
        fast.mode = 0xa000;
        fast.flags = 0;
        fast.block_map[..4].copy_from_slice(b"test");
        assert_eq!(read_symlink(&volume, 4, &fast).unwrap(), b"test");

        let mut block = vec![0_u8; BLOCK];
        block[..13].copy_from_slice(b"../etc/passwd");
        let volume = plant(&[(50, &block)]);
        let mut long = directory_inode(&leaf_root(0, 1, 50), 13);
        long.mode = 0xa000;
        assert_eq!(read_symlink(&volume, 4, &long).unwrap(), b"../etc/passwd");
    }

    #[test]
    fn test_read_symlink_rejects_a_file() {
        let volume = plant(&[]);
        let inode = directory_inode(&leaf_root(0, 1, 50), 4);
        let reason = ext4_reason(read_symlink(&volume, 4, &inode));
        assert!(reason.contains("not a symlink"), "{reason}");
    }

    #[test]
    fn test_resolve_path_on_a_real_image_follows_dotdot_without_listing_it() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/images/ext4/test.img");
        let volume = Ext4Volume::open(File::open(path).unwrap(), "test.img").unwrap();

        let root = volume.read_inode(2).unwrap();
        assert_eq!(
            names(&read_directory(&volume, 2, &root).unwrap()),
            [
                b"lost+found".as_slice(),
                b"test".as_slice(),
                b"afile".as_slice()
            ]
        );

        let (test_inode, test_dir) = resolve_path(&volume, "test").unwrap();
        assert_eq!(
            names(&read_directory(&volume, test_inode, &test_dir).unwrap()),
            [b"hello.txt".as_slice(), b"nest".as_slice()]
        );

        let (direct, _) = resolve_path(&volume, "afile").unwrap();
        let (through_parent, through_inode) = resolve_path(&volume, "test/../afile").unwrap();
        let (through_dot, _) = resolve_path(&volume, "test/./hello.txt").unwrap();
        let (hello, _) = resolve_path(&volume, "test/hello.txt").unwrap();

        assert_eq!(direct, through_parent);
        assert_eq!(through_inode.kind(), common::files::EntryKind::File);
        assert_eq!(through_dot, hello);
        assert_eq!(resolve_path(&volume, "/").unwrap().0, 2);
        assert_eq!(resolve_path(&volume, ".").unwrap().0, 2);
        assert_eq!(resolve_path(&volume, "..").unwrap().0, 2);

        assert!(matches!(
            resolve_path(&volume, "missing"),
            Err(AccessorError::NotFound { .. })
        ));
        assert!(matches!(
            resolve_path(&volume, "afile/child"),
            Err(AccessorError::NotADirectory { .. })
        ));
    }

    #[test]
    fn test_parse_directory_block_is_available_for_the_real_root() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/filesystems/ext4/debian/bookworm/root.raw");
        let block = read(path).unwrap();
        let entries = parse_directory_block(&block, 4096, true).unwrap();

        assert!(entries.iter().any(|entry| entry.name == b"lost+found"));
        assert!(entries.iter().any(|entry| entry.name == b"."));
    }
}
