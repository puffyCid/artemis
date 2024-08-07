use super::block::parse_block_bytes;
use crate::{
    artifacts::os::windows::outlook::{
        error::OutlookError, header::FormatType, pages::btree::LeafBlockData,
    },
    filesystem::ntfs::reader::read_bytes,
    utils::nom_helper::{
        nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
        nom_unsigned_two_bytes, Endian,
    },
};
use ntfs::NtfsFile;
use std::{collections::BTreeMap, io::BufReader};

pub(crate) fn parse_xblock<T: std::io::Seek + std::io::Read>(
    ntfs_file: Option<&NtfsFile<'_>>,
    fs: &mut BufReader<T>,
    block: &LeafBlockData,
    other_blocks: &[BTreeMap<u64, LeafBlockData>],
    format: &FormatType,
) -> Result<Vec<u8>, OutlookError> {
    let size = if format != &FormatType::Unicode64_4k {
        64
    } else {
        512
    };

    // Need to align block size based on Outlook file format
    let mut alignment_size = (size - block.size % size) % size;
    println!("{alignment_size}");
    let bytes = read_bytes(
        &block.block_offset,
        block.size as u64 + alignment_size as u64,
        ntfs_file,
        fs,
    )
    .unwrap();

    let (_, entries) = xblock_data(&bytes, format).unwrap();
    let mut all_bytes = Vec::new();
    for entry in entries {
        for tree in other_blocks {
            if let Some(value) = tree.get(&entry) {
                alignment_size = (size - value.size % size) % size;
                let bytes = read_bytes(
                    &value.block_offset,
                    (value.size + alignment_size) as u64,
                    ntfs_file,
                    fs,
                )
                .unwrap();

                let (_, mut block_data) = parse_block_bytes(&bytes, format).unwrap();
                all_bytes.append(&mut block_data.data);
            }
        }
    }

    Ok(all_bytes)
}

fn xblock_data<'a>(data: &'a [u8], format: &FormatType) -> nom::IResult<&'a [u8], Vec<u64>> {
    let (input, sig) = nom_unsigned_one_byte(data, Endian::Le)?;
    let (input, array_level) = nom_unsigned_one_byte(input, Endian::Le)?;
    if array_level != 1 {
        panic!("array level not 1! Its XXBLOCK!");
    }
    let (input, number_entries) = nom_unsigned_two_bytes(input, Endian::Le)?;

    let (mut input, total_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    println!("{total_size}");
    let mut count = 0;

    let mut entries = Vec::new();
    while count < number_entries {
        let (remaining, value) = if format == &FormatType::ANSI32 {
            let (bytes, value) = nom_unsigned_four_bytes(input, Endian::Le)?;
            (bytes, value as u64)
        } else {
            nom_unsigned_eight_bytes(input, Endian::Le)?
        };
        input = remaining;
        entries.push(value);
        count += 1;
    }

    Ok((input, entries))
}

#[cfg(test)]
mod tests {
    use super::{parse_xblock, xblock_data};
    use crate::{
        artifacts::os::windows::outlook::{
            header::FormatType,
            pages::btree::{get_block_btree, BlockType, LeafBlockData},
        },
        filesystem::files::file_reader,
    };
    use std::io::BufReader;

    #[test]
    fn test_parse_xblock() {
        let reader =
            file_reader("C:\\Users\\bob\\Desktop\\azur3m3m1crosoft@outlook.com.ost").unwrap();
        let mut buf_reader = BufReader::new(reader);
        let mut tree = Vec::new();

        get_block_btree(
            None,
            &mut buf_reader,
            &18800640,
            &4096,
            &FormatType::Unicode64_4k,
            &mut tree,
        )
        .unwrap();

        let test = LeafBlockData {
            index_id: 3164,
            block_type: BlockType::Internal,
            index: 470548480,
            block_offset: 507904,
            size: 65512,
            reference_count: 65512,
            file_offset_allocation_table: 2,
        };

        let bytes = parse_xblock(
            None,
            &mut buf_reader,
            &test,
            &tree,
            &FormatType::Unicode64_4k,
        )
        .unwrap();

        println!("{:?}", bytes.len());
        assert_eq!(bytes.len(), 105466)
    }

    #[test]
    fn test_xblock_data() {
        let test = [
            1, 1, 2, 0, 50, 158, 1, 0, 36, 44, 0, 0, 0, 0, 0, 0, 44, 44, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 24, 0,
            45, 236, 141, 12, 116, 94, 42, 44, 0, 0, 0, 0, 0, 0, 2, 0, 24, 0, 0, 0, 0, 0,
        ];

        let (_, entries) = xblock_data(&test, &FormatType::Unicode64_4k).unwrap();
        assert_eq!(entries.len(), 2);
    }
}
