use super::{descriptors::DescriptorData, raw::parse_raw_block, xblock::parse_xblock};
use crate::{
    artifacts::os::windows::outlook::{
        error::OutlookError,
        header::FormatType,
        helper::OutlookReader,
        pages::btree::{BlockType, LeafBlockData},
    },
    utils::{
        compression::decompress::decompress_zlib,
        nom_helper::{
            nom_data, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_two_bytes,
            Endian,
        },
    },
};
use nom::error::ErrorKind;
use ntfs::NtfsFile;
use std::{collections::BTreeMap, io::BufReader};

#[derive(Debug, Clone)]
pub(crate) struct BlockValue {
    pub(crate) block_type: Block,
    /**Set if `Block::Xblock`, `Block::Xxblock`, or `Block::Raw` */
    pub(crate) data: Vec<u8>,
    /**Set if `Block::Descriptors` */
    pub(crate) descriptors: BTreeMap<u64, DescriptorData>,
}

#[derive(PartialEq, Debug, Clone)]
pub(crate) enum Block {
    Xblock,
    Raw,
    Xxblock,
    Descriptors,
    Unknown,
}

pub(crate) trait OutlookBlock<T: std::io::Seek + std::io::Read> {
    fn parse_blocks(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        block: &LeafBlockData,
        descriptors: Option<&LeafBlockData>,
    ) -> Result<BlockValue, OutlookError>;
}

impl<'a, T: std::io::Seek + std::io::Read> OutlookBlock<T> for OutlookReader<T> {
    /// Parse the Outlook blocks to get data and/or descriptors
    fn parse_blocks(
        &mut self,
        ntfs_file: Option<&NtfsFile<'_>>,
        block: &LeafBlockData,
        descriptors: Option<&LeafBlockData>,
    ) -> Result<BlockValue, OutlookError> {
        let mut block_value = BlockValue {
            block_type: Block::Unknown,
            data: Vec::new(),
            descriptors: BTreeMap::new(),
        };

        match block.block_type {
            BlockType::Internal => parse_xblock(
                ntfs_file,
                &mut self.fs,
                block,
                &self.block_btree,
                &self.format,
                &mut block_value,
            )?,
            BlockType::External => {
                parse_raw_block(
                    ntfs_file,
                    &mut self.fs,
                    block,
                    &self.format,
                    &mut block_value,
                )?;
            }
        };

        // Not all Blocks have descriptor IDs
        if descriptors.is_some() {
            match descriptors.unwrap().block_type {
                BlockType::Internal => parse_xblock(
                    ntfs_file,
                    &mut self.fs,
                    descriptors.unwrap(),
                    &self.block_btree,
                    &self.format,
                    &mut block_value,
                )?,
                BlockType::External => {
                    parse_raw_block(
                        ntfs_file,
                        &mut self.fs,
                        descriptors.unwrap(),
                        &self.format,
                        &mut block_value,
                    )?;
                }
            }
        }

        Ok(block_value)
    }
}

#[derive(Debug)]
pub(crate) struct BlockData {
    /**If Outlook file is encrypted this data needs to be decrypted first */
    pub(crate) data: Vec<u8>,
    pub(crate) block_size: u16,
    pub(crate) sig: u16,
    pub(crate) crc: u32,
    /**Block ID (BID) */
    pub(crate) back_pointer: u64,
    pub(crate) decom_size: u32,
}
pub(crate) fn parse_block_bytes<'a>(
    data: &'a [u8],
    format: &FormatType,
) -> nom::IResult<&'a [u8], BlockData> {
    let mut block = BlockData {
        data: Vec::new(),
        block_size: 0,
        sig: 0,
        crc: 0,
        back_pointer: 0,
        decom_size: 0,
    };

    match format {
        FormatType::ANSI32 => {
            let size = 16;
            let (footer, block_data) = nom_data(data, (data.len() - size) as u64)?;

            let (input, size) = nom_unsigned_two_bytes(footer, Endian::Le)?;
            let (input, sig) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, back_pointer) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, crc) = nom_unsigned_four_bytes(input, Endian::Le)?;
            block.back_pointer = back_pointer as u64;
            block.data = block_data.to_vec();
            block.sig = sig;
            block.crc = crc;
            block.block_size = size;

            Ok((input, block))
        }
        FormatType::Unicode64 => {
            let size = 16;
            let (footer, block_data) = nom_data(data, (data.len() - size) as u64)?;

            let (input, size) = nom_unsigned_two_bytes(footer, Endian::Le)?;
            let (input, sig) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, crc) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, back_pointer) = nom_unsigned_eight_bytes(input, Endian::Le)?;

            block.back_pointer = back_pointer;
            block.data = block_data.to_vec();
            block.sig = sig;
            block.crc = crc;
            block.block_size = size;

            Ok((input, block))
        }
        FormatType::Unicode64_4k => {
            let size = 24;
            println!("length: {}", data.len());
            let (footer, block_data) = nom_data(data, (data.len() - size) as u64)?;

            let (input, size) = nom_unsigned_two_bytes(footer, Endian::Le)?;
            let (input, sig) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, crc) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, back_pointer) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (input, _unknown) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, size2) = nom_unsigned_four_bytes(input, Endian::Le)?;

            block.back_pointer = back_pointer;
            block.sig = sig;
            block.crc = crc;
            block.block_size = size;
            block.decom_size = size2;
            println!("block: {}", block.block_size);
            println!("second block: {size2}");

            if block.block_size as u32 != block.decom_size {
                println!("decom");
                // Data is compressed
                let decom_data = decompress_zlib(block_data, &None).unwrap();
                //let (_, final_bytes) = nom_data(&decom_data, block.decom_size as u64).unwrap();
                block.data = decom_data;
            } else {
                let (_, final_bytes) = nom_data(block_data, block.block_size as u64).unwrap();
                block.data = final_bytes.to_vec();
            }

            Ok((input, block))
        }
        FormatType::Unknown => {
            // We should never get here
            Err(nom::Err::Failure(nom::error::Error::new(
                data,
                ErrorKind::Fail,
            )))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::parse_block_bytes;
    use crate::{
        artifacts::os::windows::outlook::{
            blocks::block::OutlookBlock,
            header::FormatType,
            helper::{OutlookReader, OutlookReaderAction},
            pages::btree::get_block_btree,
        },
        filesystem::files::{file_reader, read_file},
    };
    use std::{io::BufReader, path::PathBuf};

    #[test]
    fn test_parse_block_bytes() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/outlook/windows11/block_raw.raw");
        let test = read_file(test_location.to_str().unwrap()).unwrap();

        let (_, results) = parse_block_bytes(&test, &FormatType::Unicode64_4k).unwrap();

        assert_eq!(results.data.len(), 456);
        assert_eq!(results.block_size, 456);
        assert_eq!(results.sig, 63926);
        assert_eq!(results.crc, 3861511615);
        assert_eq!(results.back_pointer, 69820);
    }

    #[test]
    fn test_parse_blocks() {
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

        let mut outlook_reader = OutlookReader {
            fs: buf_reader,
            block_btree: Vec::new(),
            node_btree: Vec::new(),
            format: FormatType::Unicode64_4k,
            size: 4096,
        };

        outlook_reader.setup(None).unwrap();

        for entry in &tree {
            for (_, value) in entry {
                println!("{value:?}");
                outlook_reader.parse_blocks(None, value, None).unwrap();
            }
        }
    }
}