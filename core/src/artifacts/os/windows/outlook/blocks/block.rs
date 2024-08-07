use nom::error::ErrorKind;

use crate::{
    artifacts::os::windows::outlook::header::FormatType,
    utils::{
        compression::decompress::decompress_zlib,
        nom_helper::{
            nom_data, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_two_bytes,
            Endian,
        },
    },
};

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

            return Ok((input, block));
        }
        FormatType::Unicode64 => {
            let size = 16;
            let (footer, block_data) = nom_data(data, (data.len() - size) as u64)?;

            let (input, size) = nom_unsigned_two_bytes(footer, Endian::Le)?;
            let (input, sig) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, crc) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, back_pointer) = nom_unsigned_eight_bytes(input, Endian::Le)?;

            block.back_pointer = back_pointer as u64;
            block.data = block_data.to_vec();
            block.sig = sig;
            block.crc = crc;
            block.block_size = size;

            return Ok((input, block));
        }
        FormatType::Unicode64_4k => {
            let size = 24;
            let (footer, block_data) = nom_data(data, (data.len() - size) as u64)?;

            let (input, size) = nom_unsigned_two_bytes(footer, Endian::Le)?;
            let (input, sig) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, crc) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, back_pointer) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (input, _unknown) = nom_unsigned_two_bytes(input, Endian::Le)?;
            let (input, size2) = nom_unsigned_four_bytes(input, Endian::Le)?;

            block.back_pointer = back_pointer as u64;
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

            return Ok((input, block));
        }
        FormatType::Unknown => {
            // We should never get here
            return Err(nom::Err::Failure(nom::error::Error::new(
                data,
                ErrorKind::Fail,
            )));
        }
    }
}
