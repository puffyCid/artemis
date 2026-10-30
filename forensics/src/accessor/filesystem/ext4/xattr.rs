use crate::accessor::{
    error::AccessorResult,
    filesystem::ext4::{
        data::read_file_at,
        format::{
            inode::Inode,
            superblock::{SuperBlock, ext4_error},
            xattr::{
                ExtendedAttribute, parse_attribute_block, parse_inline_attributes,
                verify_attribute_block,
            },
        },
        volume::Ext4Volume,
    },
};
use std::io::{Read, Seek};
use tracing::warn;

/// Read bytes associated with the extended attributes
///
/// Checks both inline and block data
pub(crate) fn read_extended_attribute<R>(
    volume: &Ext4Volume<R>,
    inode_number: u64,
    inode: &Inode,
) -> AccessorResult<Vec<ExtendedAttribute>>
where
    R: Read + Seek + Send,
{
    let inode_u32 = u32::try_from(inode_number).map_err(|err| {
        ext4_error(format!(
            "inode {inode_number} does not fit in 32 bits: {err:?}"
        ))
    })?;

    let mut attributes = parse_inline_attributes(inode_u32, &inode.inline_xattr)?;

    if inode.file_acl != 0 {
        let block = volume.read_block(inode.file_acl)?;
        verify_attribute_block(
            volume.superblock(),
            volume.checksum_seed(),
            inode_u32,
            inode.file_acl,
            &block,
        )?;

        attributes.extend(parse_attribute_block(inode_u32, &block)?);
    }

    // Now check for attributes that exist in a different inode
    for entry in &mut attributes {
        if entry.value_inode == 0 {
            continue;
        }

        if entry.value_inode & SuperBlock::EA_INODE != 0 {
            warn!(
                "Missing Extended Attribute flag in Superblock. Not reading attribute inode {}",
                entry.value_inode
            );
            continue;
        }

        let value = match read_attribute_value(volume, entry.value_inode, entry.value_size) {
            Ok(result) => result,
            Err(err) => {
                warn!(
                    "inode {inode_u32} attribute value inode {} was not read: {err:?}",
                    entry.value_inode
                );
                continue;
            }
        };

        entry.value = value;
    }

    Ok(attributes)
}

/// Read attributes that exist on a separate inode number
fn read_attribute_value<R>(
    volume: &Ext4Volume<R>,
    value_inode: u32,
    value_size: u32,
) -> AccessorResult<Vec<u8>>
where
    R: Read + Seek + Send,
{
    let inode = volume.read_inode(value_inode as u64)?;
    if inode.size != value_size as u64 {
        warn!(
            "inode {value_inode} attribute value is {value_size} bytes, inode size is {}",
            inode.size
        );
    }

    let buf_size = usize::try_from(value_size).map_err(|err| {
        ext4_error(format!(
            "inode {value_inode} attribute value size does not fit in memory: {err:?}"
        ))
    })?;

    let mut buf = vec![0_u8; buf_size];

    let read = read_file_at(volume, value_inode as u64, &inode, 0, &mut buf)?;
    if read != buf_size {
        warn!("inode {value_inode} attribute value read {read} of {buf_size} bytes");
    }

    buf.truncate(read);

    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::read_extended_attribute;
    use crate::accessor::filesystem::ext4::{
        format::{checksum::crc32c_check, inode::parse_inode, xattr::parse_inline_attributes},
        volume::Ext4Volume,
    };
    use std::{
        fs::{File, read},
        io::Cursor,
        path::PathBuf,
    };

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

    fn plant(block_number: u64, contents: &[u8]) -> Ext4Volume<Cursor<Vec<u8>>> {
        let superblock = one_group_superblock();
        let descriptor = group0();
        let mut image = vec![0_u8; 8192];

        image[1024..2048].copy_from_slice(&superblock);
        image[4096..4096 + descriptor.len()].copy_from_slice(&descriptor);
        let offset = usize::try_from(block_number).unwrap() * 4096;
        let end = offset + contents.len();

        if image.len() < end {
            image.resize(end, 0);
        }

        image[offset..end].copy_from_slice(contents);
        Ext4Volume::open(Cursor::new(image), "bookworm").unwrap()
    }

    fn put_u16(data: &mut [u8], offset: usize, value: u16) {
        data[offset..offset + 2].copy_from_slice(&value.to_le_bytes());
    }

    fn put_u32(data: &mut [u8], offset: usize, value: u32) {
        data[offset..offset + 4].copy_from_slice(&value.to_le_bytes());
    }

    fn external_block(seed: u32, block_number: u64) -> Vec<u8> {
        let mut data = vec![0_u8; 4096];
        put_u32(&mut data, 0, 0xea020000);
        put_u32(&mut data, 4, 1);
        put_u32(&mut data, 8, 1);

        data[32] = 3;
        data[33] = 1;

        put_u16(&mut data, 34, 4088);
        put_u32(&mut data, 40, 4);
        data[48..51].copy_from_slice(b"big");
        data[4088..4092].copy_from_slice(b"DATA");

        let mut actual = crc32c_check(seed, &block_number.to_le_bytes());
        actual = crc32c_check(actual, &data);
        put_u32(&mut data, 16, actual);

        data
    }

    #[test]
    fn test_read_extended_attribute_reads_selinux_from_a_real_inode() {
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        path.push("tests/test_data/images/ext4/test.img");
        let volume = Ext4Volume::open(File::open(path).unwrap(), "test.img").unwrap();

        let root = volume.read_inode(2).unwrap();
        assert!(
            read_extended_attribute(&volume, 2, &root)
                .unwrap()
                .is_empty()
        );

        let afile = volume.read_inode(18).unwrap();
        let attributes = read_extended_attribute(&volume, 18, &afile).unwrap();
        assert_eq!(attributes.len(), 1);
        assert_eq!(attributes[0].name, b"security.selinux");
        assert_eq!(
            attributes[0].value,
            b"unconfined_u:object_r:unlabeled_t:s0\0"
        );
    }

    #[test]
    fn test_read_extended_attribute_reads_an_external_block() {
        let volume = plant(0, &[]);
        let seed = volume.checksum_seed().unwrap();

        let block = external_block(seed, 50);
        let volume = plant(50, &block);

        let mut inode = parse_inode(&[0_u8; 256], 256, true, true, true).unwrap();
        inode.inline_xattr.clear();
        inode.file_acl = 50;

        let attributes = read_extended_attribute(&volume, 4, &inode).unwrap();

        assert_eq!(attributes.len(), 1);
        assert_eq!(attributes[0].name, b"user.big");
        assert_eq!(attributes[0].value, b"DATA");
        assert!(
            parse_inline_attributes(4, &inode.inline_xattr)
                .unwrap()
                .is_empty()
        );
    }
}
