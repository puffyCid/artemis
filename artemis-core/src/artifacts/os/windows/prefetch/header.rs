use crate::utils::{
    nom_helper::{nom_unsigned_four_bytes, Endian},
    strings::extract_utf16_string,
};
use nom::bytes::complete::take;
use serde::Serialize;

pub(crate) struct CompressedHeader {
    _signature: u32,
    pub(crate) uncompressed_size: u32,
}

impl CompressedHeader {
    /// Parse compressed prefetch header. Default since Windows 8
    pub(crate) fn parse_compressed_header(data: &[u8]) -> nom::IResult<&[u8], CompressedHeader> {
        let (input, signature) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, uncompressed_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let header = CompressedHeader {
            _signature: signature,
            uncompressed_size,
        };

        Ok((input, header))
    }

    /// Check for prefetch signature (MAM - Compressed)
    pub(crate) fn is_compressed(data: &[u8]) -> nom::IResult<&[u8], bool> {
        let (input, signature) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let compressed_sig = 0x44d414d; // MAM
        if signature == compressed_sig {
            return Ok((input, true));
        }
        Ok((input, false))
    }
}

#[derive(Debug, Serialize)]
pub(crate) struct Header {
    pub(crate) version: u32,
    signature: u32,
    unknown: u32,
    pub(crate) size: u32,
    pub(crate) filename: String,
    pub(crate) pf_hash: String,
    unknown_flags: u32,
}

impl Header {
    pub(crate) fn parse_header(data: &[u8]) -> nom::IResult<&[u8], Header> {
        let (input, version) = nom_unsigned_four_bytes(data, Endian::Le)?;
        let (input, signature) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _unknown_data) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let filename_size: usize = 60;
        let (input, filename_data) = take(filename_size)(input)?;
        let (input, pf_hash) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _unknown_flags_data) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let header = Header {
            version,
            signature,
            unknown: 0,
            size,
            filename: extract_utf16_string(filename_data),
            pf_hash: format!("{pf_hash:X?}"),
            unknown_flags: 0,
        };

        Ok((input, header))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::prefetch::header::Header,
        utils::compression::decompress::{decompress_xpress, XpressType},
    };

    use super::CompressedHeader;
    use std::{fs, path::PathBuf};

    #[test]
    fn test_parse_compressed_header() {
        let test_data = vec![77, 65, 77, 4, 116, 199, 0, 0];

        let (_, result) = CompressedHeader::parse_compressed_header(&test_data).unwrap();
        assert_eq!(result._signature, 0x44d414d); // MAM
        assert_eq!(result.uncompressed_size, 51060);
    }

    #[test]
    fn test_is_compressed() {
        let test_data = vec![77, 65, 77, 4, 116, 199, 0, 0];

        let (_, result) = CompressedHeader::is_compressed(&test_data).unwrap();
        assert_eq!(result, true);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_parse_header_api() {
        use crate::utils::compression::xpress::api::decompress_huffman_api;

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/win11/7Z.EXE-886612C8.pf");

        let buffer = fs::read(test_location).unwrap();

        let (data, header) = CompressedHeader::parse_compressed_header(&buffer).unwrap();
        assert_eq!(header._signature, 0x44d414d);
        assert_eq!(header.uncompressed_size, 51060);
        let huffman = 4;

        let result = decompress_huffman_api(
            &mut data.to_vec(),
            &XpressType::XpressHuffman,
            header.uncompressed_size,
        )
        .unwrap();

        assert_eq!(result.len(), 51060);

        let (_, result) = Header::parse_header(&result).unwrap();
        assert_eq!(result.version, 30);
        assert_eq!(result.signature, 0x41434353); // SCCA
        assert_eq!(result.unknown, 0);
        assert_eq!(result.filename, "7Z.EXE");
        assert_eq!(result.pf_hash, "886612C8");
        assert_eq!(result.unknown_flags, 0);
    }

    #[test]
    fn test_parse_header() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/win11/7Z.EXE-886612C8.pf");

        let buffer = fs::read(test_location).unwrap();

        let (data, header) = CompressedHeader::parse_compressed_header(&buffer).unwrap();
        assert_eq!(header._signature, 0x44d414d);
        assert_eq!(header.uncompressed_size, 51060);
        let huffman = 4;

        let result = decompress_xpress(
            &mut data.to_vec(),
            header.uncompressed_size,
            &XpressType::XpressHuffman,
        )
        .unwrap();

        assert_eq!(result.len(), 51060);

        let (_, result) = Header::parse_header(&result).unwrap();
        assert_eq!(result.version, 30);
        assert_eq!(result.signature, 0x41434353); // SCCA
        assert_eq!(result.unknown, 0);
        assert_eq!(result.filename, "7Z.EXE");
        assert_eq!(result.pf_hash, "886612C8");
        assert_eq!(result.unknown_flags, 0);
    }
}
