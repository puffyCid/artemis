use super::{
    error::CompressionError,
    xpress::{huffman::decompress_xpress_huffman, lz77::decompress_lz77, lznt::decompress_lznt},
};
use crate::filesystem::files::read_file;
use flate2::{
    bufread::{MultiGzDecoder, ZlibDecoder},
    Decompress,
};
use log::{error, warn};
use lz4_flex::block::decompress_with_dict;
use ruzstd::StreamingDecoder;
use std::io::Read;
use xz2::read::XzDecoder;

/// Decompress gzip compressed file
pub(crate) fn decompress_gzip(path: &str) -> Result<Vec<u8>, CompressionError> {
    let buffer_result = read_file(path);
    let buffer = match buffer_result {
        Ok(result) => result,
        Err(err) => {
            error!("[compression] Could not read file {path}: {err:?}");
            return Err(CompressionError::GzipReadFile);
        }
    };
    decompress_gzip_data(&buffer)
}

/// Decompress raw gzip bytes
pub(crate) fn decompress_gzip_data(buffer: &[u8]) -> Result<Vec<u8>, CompressionError> {
    let mut data = MultiGzDecoder::new(buffer);

    let mut decompress_data = Vec::new();
    let result = data.read_to_end(&mut decompress_data);
    if result.is_err() {
        error!(
            "[compression] Could not decompress data: {:?}",
            result.unwrap_err()
        );
        return Err(CompressionError::GzipDecompress);
    }

    Ok(decompress_data)
}

/// Decompress zstd data
pub(crate) fn decompress_zstd(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    let decoder_result = StreamingDecoder::new(data);
    let mut decoder = match decoder_result {
        Ok(result) => result,
        Err(err) => {
            error!("[compresssion] Could not decompress zstd data: {err:?}");
            return Err(CompressionError::ZstdDecompresss);
        }
    };
    let mut data = Vec::new();
    if decoder.read_to_end(&mut data).is_err() {
        return Err(CompressionError::ZstdDecompresss);
    }
    Ok(data)
}

/// Decompress lz4 data
pub(crate) fn decompress_lz4(
    data: &[u8],
    decom_size: usize,
    initial_dict: &[u8],
) -> Result<Vec<u8>, CompressionError> {
    let decompress_result = decompress_with_dict(data, decom_size, initial_dict);
    let decomp_data = match decompress_result {
        Ok(result) => result,
        Err(err) => {
            error!("[compression] Could not decompress lz4 data: {err:?}");
            return Err(CompressionError::Lz4Decompresss);
        }
    };
    Ok(decomp_data)
}

/// Attemp to decompress zlib raw data (no header)
pub(crate) fn decompress_zlib(
    data: &[u8],
    wbits: &Option<u8>,
) -> Result<Vec<u8>, CompressionError> {
    let mut buffer = if wbits.is_some() {
        let wbits_value = wbits.unwrap_or_default();
        let min_size = 9;
        let max_size = 15;
        if wbits_value < min_size || wbits_value > max_size {
            return Err(CompressionError::ZlibBadWbits);
        }

        ZlibDecoder::new_with_decompress(data, Decompress::new_with_window_bits(false, wbits_value))
    } else {
        ZlibDecoder::new(data)
    };
    let mut decompress_data = Vec::new();

    let result = buffer.read_to_end(&mut decompress_data);
    if result.is_err() {
        error!(
            "[compression] Could not decompress zlib data: {:?}",
            result.unwrap_err()
        );
        return Err(CompressionError::ZlibDecompress);
    }

    Ok(decompress_data)
}

/// Decompress xz data
pub(crate) fn decompress_xz(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    let mut decompress = XzDecoder::new(data);
    let mut data: Vec<u8> = Vec::new();
    if decompress.read_to_end(&mut data).is_err() {
        error!("[compression] Could not decompress xz data");
        return Err(CompressionError::XzDecompress);
    }

    Ok(data)
}

/// Decompress seven bit compression
pub(crate) fn decompress_seven_bit(data: &[u8]) -> Vec<u8> {
    let mut decompressed_data: Vec<u8> = Vec::new();
    let mut index: u16 = 0;
    let mut bit_value = 0;

    let max_value = 7;
    let bit_op = 0x7f;

    for value in data {
        bit_value |= (value.to_owned() as u16) << index;
        decompressed_data.push((bit_value & bit_op) as u8);
        bit_value >>= max_value;

        index += 1;

        if index == max_value {
            decompressed_data.push((bit_value & bit_op) as u8);
            bit_value >>= max_value;
            index = 0;
        }
    }
    decompressed_data
}

pub(crate) enum XpressType {
    XpressHuffman,
    Lz77,
    _Lznt,
    _Default,
    _None,
}

/// Decompress XPRESS compressed data
pub(crate) fn decompress_xpress(
    data: &mut [u8],
    decompress_size: u32,
    format: &XpressType,
) -> Result<Vec<u8>, CompressionError> {
    let mut decompress_data: Vec<u8> = Vec::with_capacity(decompress_size as usize);
    match format {
        XpressType::XpressHuffman => decompress_xpress_huffman(data, &mut decompress_data)?,
        XpressType::Lz77 => decompress_lz77(data, &mut decompress_data)?,
        XpressType::_Lznt => decompress_lznt(data, &mut decompress_data)?,
        XpressType::_Default => {
            warn!("[compression] Default type unsupported");
            return Err(CompressionError::HuffmanCompressionDefault);
        }
        XpressType::_None => {
            warn!("[compression] None type unsupported");
            return Err(CompressionError::HuffmanCompressionNone);
        }
    }

    Ok(decompress_data)
}

/**
 * Decomress RTF compressed data. This is found mainly in Microsoft Outlook.
 * Inspired by https://github.com/delimitry/compressed_rtf/blob/master/compressed_rtf/compressed_rtf.py (MIT license)
 */
pub(crate) fn decompress_rtf(data: &[u8], decom_size: &u32) -> Result<Vec<u8>, CompressionError> {
    let intial_string = "{\\rtf1\\ansi\\mac\\deff0\\deftab720{\\fonttbl;}{\\f0\\fnil \\froman \\fswiss \\fmodern \\fscript \\fdecor MS Sans SerifSymbolArialTimes New RomanCourier{\\colortbl\\red0\\green0\\blue0\n\r\\par \\pard\\plain\\f0\\fs20\\b\\i\\u\\tab\\tx".as_bytes();
    const MAX_LZ_REFERENCE: usize = 4096;
    // Size of the intial string above
    const SIZE: usize = 207;
    let mut initial_buf = [0; (MAX_LZ_REFERENCE - SIZE)];
    initial_buf.fill(0);

    let mut start = [intial_string, &initial_buf].concat();

    let mut decom_data = Vec::new();
    let mut buf_position = SIZE;

    let mut position = 0;

    let mut done = false;
    while !done {
        if position > data.len() {
            warn!(
                "[compression] Data position greater than data size: {position} vs {}",
                data.len()
            );
            break;
        }
        let bit = data[position];
        position += 1;

        let bits = format!("{0:08b}", bit);
        let bit_string = bits.chars().rev();
        for entry in bit_string {
            if entry == '1' {
                if position + 2 > data.len() {
                    warn!(
                        "[compression] Data reference position greater than data size: {} vs {}",
                        position + 2,
                        data.len()
                    );
                    done = true;
                    break;
                }
                let ref_offset = &data[position..position + 2];
                position += 2;

                let ref_value = [ref_offset[0], ref_offset[1]];
                let mut offset = u16::from_be_bytes(ref_value);

                let size = offset & 0b1111;

                offset >>= 4;
                offset &= 0b111111111111;

                if buf_position == offset as usize {
                    done = true;
                    break;
                }

                for value in 0..size + 2 {
                    let value_offset = (offset + value) as usize % MAX_LZ_REFERENCE;
                    if value_offset > start.len() {
                        warn!(
                            "[compression] Value offset greater than start size: {} vs {}",
                            value_offset,
                            start.len()
                        );
                        break;
                    }
                    let value = start[value_offset];
                    decom_data.push(value);

                    if buf_position > start.len() {
                        warn!(
                            "[compression] Buffer position greater than start size: {buf_position} vs {}",
                            start.len()
                        );
                        break;
                    }

                    start[buf_position] = value;
                    buf_position = (buf_position + 1) % MAX_LZ_REFERENCE;
                }
                continue;
            }

            if position > data.len() {
                warn!(
                    "[compression] Data position greater than data size, cannot get next byte: {position} vs {}",
                    data.len()
                );
                done = true;
                break;
            }

            let next_bit = data[position];
            position += 1;
            decom_data.push(next_bit);
            if buf_position > start.len() {
                warn!(
                    "[compression] Buffer position greater than start size, cannot set next byte: {buf_position} vs {}",
                    start.len()
                );
                done = true;
                break;
            }
            start[buf_position] = next_bit;
            buf_position = (buf_position + 1) % MAX_LZ_REFERENCE;
        }
    }

    if decom_data.len() as u32 != *decom_size {
        error!("[compression] Failed to decompress RTF data expected decompress size {decom_size} got {}", decom_data.len());
        return Err(CompressionError::RtfCorrupted);
    }

    Ok(decom_data)
}

#[cfg(test)]
mod tests {
    use super::decompress_rtf;
    use crate::{
        filesystem::files::read_file,
        utils::{
            compression::decompress::{
                decompress_gzip, decompress_gzip_data, decompress_lz4, decompress_seven_bit,
                decompress_xpress, decompress_xz, decompress_zlib, decompress_zstd, XpressType,
            },
            nom_helper::{nom_unsigned_four_bytes, Endian},
        },
    };
    use std::path::PathBuf;

    #[test]
    fn test_decompress_gzip() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/fsevents/DLS2/0000000000027d79");
        let files = decompress_gzip(&test_location.display().to_string()).unwrap();
        assert_eq!(files.len(), 78970);
    }

    #[test]
    #[should_panic(expected = "GzipDecompress")]
    fn test_decompress_gzip_data_error() {
        let test_data = [
            40, 181, 47, 253, 96, 246, 1, 13, 11, 0, 38, 86, 70, 34, 48, 79, 220, 104, 104, 164,
            213, 236, 199, 164, 19, 243, 36, 222, 54, 232, 158, 27, 205, 124, 87, 133, 215, 237,
            160, 61, 33, 255, 131, 30, 20, 52, 81, 42, 62, 0, 59, 0, 61, 0, 11, 131, 33, 196, 25,
            210, 75, 130, 113, 33, 146, 97, 53, 111, 247, 103, 181, 12, 105, 217, 149, 183, 112,
            45, 173, 22, 87, 62, 248, 203, 155, 98, 89, 184, 157, 136, 113, 221, 146, 112, 201,
            134, 243, 130, 93, 228, 89, 237, 17, 241, 120, 60, 11, 106, 48, 120, 20, 14, 32, 232,
            121, 8, 213, 244, 202, 19, 135, 53, 1, 7, 160, 125, 197, 162, 176, 210, 24, 121, 122,
            101, 170, 229, 211, 231, 173, 170, 141, 157, 87, 253, 236, 14, 158, 59, 33, 136, 159,
            220, 201, 69, 73, 207, 165, 0, 193, 32, 17, 177, 0, 45, 170, 157, 2, 190, 92, 159, 75,
            98, 244, 244, 1, 77, 227, 73, 168, 84, 187, 37, 194, 165, 222, 18, 121, 17, 7, 41, 135,
            210, 83, 153, 126, 37, 125, 217, 240, 37, 207, 8, 43, 236, 59, 189, 198, 117, 29, 23,
            205, 152, 178, 154, 168, 38, 117, 20, 232, 3, 193, 59, 97, 194, 197, 72, 2, 88, 99,
            154, 129, 165, 160, 60, 184, 8, 73, 195, 26, 67, 208, 176, 2, 231, 185, 14, 203, 195,
            105, 42, 199, 247, 122, 70, 142, 0, 207, 101, 119, 81, 210, 96, 205, 97, 19, 82, 28,
            37, 0, 1, 173, 193, 176, 143, 148, 189, 157, 62, 199, 74, 106, 74, 191, 226, 189, 115,
            148, 228, 46, 68, 9, 99, 90, 19, 25, 0, 8, 48, 179, 128, 201, 15, 71, 22, 170, 254, 39,
            8, 216, 246, 107, 136, 75, 38, 214, 245, 184, 88, 200, 89, 197, 179, 101, 209, 103,
            196, 201, 9, 27, 133, 6, 11, 67, 204, 216, 132, 63, 226, 133, 45, 4, 177, 5, 85, 18,
            182, 230, 176, 178, 215, 245, 107, 134, 127, 83, 173, 195, 245, 106, 25, 9, 33, 10,
        ];
        let _ = decompress_gzip_data(&test_data).unwrap();
    }

    #[test]
    fn test_decompress_zstd() {
        let test_data = [
            40, 181, 47, 253, 96, 246, 1, 13, 11, 0, 38, 86, 70, 34, 48, 79, 220, 104, 104, 164,
            213, 236, 199, 164, 19, 243, 36, 222, 54, 232, 158, 27, 205, 124, 87, 133, 215, 237,
            160, 61, 33, 255, 131, 30, 20, 52, 81, 42, 62, 0, 59, 0, 61, 0, 11, 131, 33, 196, 25,
            210, 75, 130, 113, 33, 146, 97, 53, 111, 247, 103, 181, 12, 105, 217, 149, 183, 112,
            45, 173, 22, 87, 62, 248, 203, 155, 98, 89, 184, 157, 136, 113, 221, 146, 112, 201,
            134, 243, 130, 93, 228, 89, 237, 17, 241, 120, 60, 11, 106, 48, 120, 20, 14, 32, 232,
            121, 8, 213, 244, 202, 19, 135, 53, 1, 7, 160, 125, 197, 162, 176, 210, 24, 121, 122,
            101, 170, 229, 211, 231, 173, 170, 141, 157, 87, 253, 236, 14, 158, 59, 33, 136, 159,
            220, 201, 69, 73, 207, 165, 0, 193, 32, 17, 177, 0, 45, 170, 157, 2, 190, 92, 159, 75,
            98, 244, 244, 1, 77, 227, 73, 168, 84, 187, 37, 194, 165, 222, 18, 121, 17, 7, 41, 135,
            210, 83, 153, 126, 37, 125, 217, 240, 37, 207, 8, 43, 236, 59, 189, 198, 117, 29, 23,
            205, 152, 178, 154, 168, 38, 117, 20, 232, 3, 193, 59, 97, 194, 197, 72, 2, 88, 99,
            154, 129, 165, 160, 60, 184, 8, 73, 195, 26, 67, 208, 176, 2, 231, 185, 14, 203, 195,
            105, 42, 199, 247, 122, 70, 142, 0, 207, 101, 119, 81, 210, 96, 205, 97, 19, 82, 28,
            37, 0, 1, 173, 193, 176, 143, 148, 189, 157, 62, 199, 74, 106, 74, 191, 226, 189, 115,
            148, 228, 46, 68, 9, 99, 90, 19, 25, 0, 8, 48, 179, 128, 201, 15, 71, 22, 170, 254, 39,
            8, 216, 246, 107, 136, 75, 38, 214, 245, 184, 88, 200, 89, 197, 179, 101, 209, 103,
            196, 201, 9, 27, 133, 6, 11, 67, 204, 216, 132, 63, 226, 133, 45, 4, 177, 5, 85, 18,
            182, 230, 176, 178, 215, 245, 107, 134, 127, 83, 173, 195, 245, 106, 25, 9, 33, 10,
        ];
        let result = decompress_zstd(&test_data).unwrap();
        assert_eq!(result.len(), 758);
    }

    #[test]
    fn test_decompress_xz() {
        let test_data = [
            253, 55, 122, 88, 90, 0, 0, 0, 255, 18, 217, 65, 3, 192, 169, 3, 131, 8, 33, 1, 16, 0,
            0, 0, 211, 65, 240, 142, 224, 4, 2, 1, 161, 93, 0, 33, 147, 198, 137, 174, 252, 60, 42,
            46, 195, 29, 60, 248, 205, 175, 108, 16, 165, 45, 1, 213, 149, 248, 139, 156, 158, 105,
            187, 7, 81, 7, 83, 19, 97, 130, 4, 93, 160, 13, 49, 168, 225, 0, 97, 23, 97, 47, 127,
            26, 196, 167, 250, 7, 218, 185, 125, 28, 130, 124, 136, 220, 80, 181, 67, 25, 170, 234,
            69, 55, 209, 98, 15, 30, 131, 65, 191, 37, 197, 171, 228, 54, 232, 107, 93, 65, 193, 7,
            93, 142, 17, 193, 87, 108, 95, 219, 94, 155, 59, 118, 250, 190, 124, 7, 139, 26, 54,
            234, 214, 11, 85, 16, 198, 239, 117, 142, 188, 138, 0, 73, 71, 183, 118, 151, 145, 109,
            205, 48, 222, 249, 66, 113, 102, 147, 74, 121, 83, 72, 151, 66, 82, 126, 140, 77, 7,
            55, 170, 29, 67, 13, 202, 177, 118, 58, 29, 95, 8, 60, 238, 211, 135, 249, 130, 242, 5,
            91, 158, 254, 132, 145, 185, 35, 185, 87, 103, 141, 60, 231, 31, 140, 237, 97, 34, 91,
            222, 151, 130, 141, 195, 73, 55, 239, 7, 211, 45, 185, 57, 218, 119, 157, 113, 171, 27,
            75, 237, 41, 204, 198, 27, 73, 237, 251, 121, 0, 248, 118, 213, 174, 65, 126, 134, 151,
            203, 201, 56, 136, 176, 237, 80, 239, 19, 126, 60, 239, 99, 247, 59, 50, 119, 182, 234,
            174, 38, 66, 39, 206, 254, 244, 243, 231, 85, 118, 120, 26, 7, 176, 127, 211, 212, 203,
            15, 6, 187, 220, 46, 62, 126, 176, 113, 230, 70, 218, 163, 63, 20, 100, 184, 29, 132,
            104, 74, 186, 38, 225, 167, 156, 24, 232, 223, 198, 44, 4, 229, 152, 250, 221, 76, 139,
            156, 38, 127, 156, 57, 163, 233, 94, 167, 219, 95, 8, 12, 108, 34, 159, 167, 26, 248,
            164, 93, 121, 247, 249, 48, 82, 211, 162, 215, 238, 53, 157, 126, 246, 211, 209, 67,
            150, 240, 196, 25, 204, 46, 52, 205, 249, 182, 88, 53, 66, 142, 237, 174, 164, 172, 24,
            89, 86, 86, 218, 2, 33, 68, 63, 182, 204, 190, 198, 95, 99, 30, 82, 162, 163, 100, 128,
            136, 46, 213, 27, 74, 170, 202, 197, 44, 150, 250, 16, 250, 101, 7, 60, 203, 161, 90,
            127, 210, 56, 140, 121, 205, 245, 233, 86, 154, 180, 167, 107, 82, 228, 188, 244, 121,
            19, 81, 100, 18, 90, 77, 46, 0, 0, 0, 0, 0, 1, 185, 3, 131, 8, 0, 0, 221, 164, 207, 72,
            168, 0, 10, 252, 2, 0, 0, 0, 0, 0, 89, 90,
        ];
        let result = decompress_xz(&test_data).unwrap();
        assert_eq!(result.len(), 1027);
    }

    #[test]
    fn test_decompress_lz4() {
        let test_data = [
            255, 157, 77, 69, 83, 83, 65, 71, 69, 61, 74, 83, 32, 69, 82, 82, 79, 82, 58, 32, 69,
            120, 99, 101, 112, 116, 105, 111, 110, 32, 105, 110, 32, 99, 97, 108, 108, 98, 97, 99,
            107, 32, 102, 111, 114, 32, 115, 105, 103, 110, 97, 108, 58, 32, 112, 111, 115, 105,
            116, 105, 111, 110, 45, 99, 104, 97, 110, 103, 101, 100, 58, 32, 84, 121, 112, 101, 69,
            114, 114, 111, 114, 58, 32, 116, 104, 105, 115, 46, 95, 114, 101, 99, 116, 32, 105,
            115, 32, 110, 117, 108, 108, 10, 103, 101, 116, 67, 117, 114, 114, 101, 110, 116, 82,
            101, 99, 116, 64, 114, 101, 115, 111, 117, 114, 99, 101, 58, 47, 47, 47, 111, 114, 103,
            47, 103, 110, 111, 109, 101, 47, 115, 104, 101, 108, 108, 47, 117, 105, 47, 107, 101,
            121, 98, 111, 97, 114, 100, 46, 106, 115, 58, 53, 54, 49, 58, 50, 50, 10, 119, 114, 97,
            112, 112, 101, 114, 58, 0, 4, 243, 23, 103, 106, 115, 47, 109, 111, 100, 117, 108, 101,
            115, 47, 95, 108, 101, 103, 97, 99, 121, 46, 106, 115, 58, 56, 50, 58, 50, 50, 10, 95,
            111, 110, 70, 111, 99, 117, 115, 80, 180, 0, 18, 67, 179, 0, 15, 75, 0, 4, 15, 133, 0,
            2, 111, 54, 52, 51, 58, 50, 48, 133, 0, 42, 79, 101, 109, 105, 116, 115, 0, 4, 8, 190,
            0, 2, 97, 1, 229, 115, 46, 106, 115, 58, 49, 50, 56, 58, 50, 55, 10, 95, 115, 60, 1,
            114, 87, 105, 110, 100, 111, 119, 47, 96, 1, 18, 99, 80, 1, 2, 20, 0, 4, 220, 0, 63,
            73, 100, 60, 101, 0, 4, 15, 216, 0, 2, 112, 53, 51, 53, 58, 50, 49, 10,
        ];
        let result = decompress_lz4(&test_data, 514, &[]).unwrap();
        assert_eq!(result.len(), 514);
    }

    #[test]
    fn test_decompress_seven_bit() {
        let test = [213, 121, 89, 62, 7];
        let result = decompress_seven_bit(&test);
        assert_eq!(result, [85, 115, 101, 114, 115]);
    }

    #[test]
    fn test_decompress_zlib() {
        let test = [
            120, 156, 5, 128, 209, 9, 0, 0, 4, 68, 87, 97, 56, 229, 227, 149, 194, 237, 127, 117,
            193, 196, 234, 62, 13, 25, 218, 4, 36,
        ];
        let result = decompress_zlib(&test, &None).unwrap();
        assert_eq!(
            result,
            [104, 101, 108, 108, 111, 32, 114, 117, 115, 116, 33]
        );
    }

    #[test]
    fn test_decompress_xpress() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/compression/lz_huffman.raw");
        let mut bytes = read_file(&test_location.display().to_string()).unwrap();
        let decom_data = decompress_xpress(&mut bytes, 153064, &XpressType::XpressHuffman).unwrap();
        assert_eq!(decom_data.len(), 153064);
    }

    #[test]
    fn test_decompress_rtf() {
        let test = [
            219, 0, 0, 0, 71, 1, 0, 0, 76, 90, 70, 117, 83, 82, 121, 25, 97, 0, 10, 102, 98, 105,
            100, 4, 0, 0, 99, 99, 192, 112, 103, 49, 50, 53, 50, 0, 254, 3, 67, 240, 116, 101, 120,
            116, 1, 247, 2, 164, 3, 227, 2, 0, 4, 99, 104, 10, 192, 115, 101, 116, 48, 32, 239, 7,
            109, 2, 131, 0, 80, 17, 77, 50, 10, 128, 6, 180, 2, 128, 150, 125, 10, 128, 8, 200, 59,
            9, 98, 49, 57, 14, 192, 191, 9, 195, 22, 114, 10, 50, 22, 113, 2, 128, 21, 98, 42, 9,
            176, 115, 9, 240, 4, 144, 97, 116, 5, 178, 14, 80, 3, 96, 115, 162, 111, 1, 128, 32,
            69, 120, 17, 193, 110, 24, 48, 93, 6, 82, 118, 4, 144, 23, 182, 2, 16, 114, 0, 192,
            116, 125, 8, 80, 110, 26, 49, 16, 32, 5, 192, 5, 160, 27, 100, 100, 154, 32, 3, 82, 32,
            16, 34, 23, 178, 92, 118, 8, 144, 228, 119, 107, 11, 128, 100, 53, 29, 83, 4, 240, 7,
            64, 13, 23, 112, 48, 10, 113, 23, 242, 98, 107, 109, 107, 6, 115, 1, 144, 0, 32, 32,
            66, 77, 95, 66, 224, 69, 71, 73, 78, 125, 10, 252, 21, 81, 33, 96,
        ];
        let (input, _compression_size) = nom_unsigned_four_bytes(&test, Endian::Le).unwrap();
        let (input, uncompressed_size) = nom_unsigned_four_bytes(input, Endian::Le).unwrap();
        let (input, _sig) = nom_unsigned_four_bytes(input, Endian::Le).unwrap();
        let (input, _crc) = nom_unsigned_four_bytes(input, Endian::Le).unwrap();
        let result = decompress_rtf(input, &uncompressed_size).unwrap();

        assert_eq!(result.len(), uncompressed_size as usize);
        assert_eq!(
            result,
            vec![
                123, 92, 114, 116, 102, 49, 92, 97, 110, 115, 105, 92, 102, 98, 105, 100, 105, 115,
                92, 97, 110, 115, 105, 99, 112, 103, 49, 50, 53, 50, 92, 100, 101, 102, 102, 48,
                92, 100, 101, 102, 116, 97, 98, 55, 50, 48, 92, 102, 114, 111, 109, 116, 101, 120,
                116, 123, 92, 102, 111, 110, 116, 116, 98, 108, 123, 92, 102, 48, 92, 102, 115,
                119, 105, 115, 115, 92, 102, 99, 104, 97, 114, 115, 101, 116, 48, 32, 84, 105, 109,
                101, 115, 32, 78, 101, 119, 32, 82, 111, 109, 97, 110, 59, 125, 123, 92, 102, 49,
                92, 102, 115, 119, 105, 115, 115, 92, 102, 99, 104, 97, 114, 115, 101, 116, 50, 10,
                13, 83, 121, 109, 98, 111, 108, 59, 125, 125, 10, 13, 123, 92, 99, 111, 108, 111,
                114, 116, 98, 108, 59, 92, 114, 101, 100, 49, 57, 50, 92, 103, 114, 101, 101, 110,
                49, 57, 50, 92, 98, 108, 117, 101, 49, 57, 50, 59, 125, 10, 13, 123, 92, 42, 92,
                103, 101, 110, 101, 114, 97, 116, 111, 114, 32, 77, 105, 99, 114, 111, 115, 111,
                102, 116, 32, 69, 120, 99, 104, 97, 110, 103, 101, 32, 83, 101, 114, 118, 101, 114,
                59, 125, 10, 13, 123, 92, 42, 92, 102, 111, 114, 109, 97, 116, 67, 111, 110, 118,
                101, 114, 116, 101, 114, 32, 99, 111, 110, 118, 101, 114, 116, 101, 100, 32, 102,
                114, 111, 109, 32, 116, 101, 120, 116, 59, 125, 10, 13, 92, 118, 105, 101, 119,
                107, 105, 110, 100, 53, 92, 118, 105, 101, 119, 115, 99, 97, 108, 101, 49, 48, 48,
                10, 13, 123, 92, 42, 92, 98, 107, 109, 107, 115, 116, 97, 114, 116, 32, 66, 77, 95,
                66, 69, 71, 73, 78, 125, 92, 112, 97, 114, 100, 92, 112, 108, 97, 105, 110, 92,
                102, 48, 125, 10, 13
            ]
        );
    }

    #[test]
    #[should_panic(expected = "RtfCorrupted")]
    fn test_decompress_rtf_corrupt() {
        let test = [
            219, 0, 0, 0, 71, 1, 0, 0, 76, 90, 70, 117, 83, 82, 121, 25, 97, 0, 10, 102, 98, 105,
            100, 4, 0, 0, 99, 99, 192, 112, 103, 49, 50, 53, 50, 0, 254, 3, 67, 240, 116, 101, 120,
            116, 1, 247, 2, 164, 3, 22,
        ];
        let (input, _compression_size) = nom_unsigned_four_bytes(&test, Endian::Le).unwrap();
        let (input, uncompressed_size) = nom_unsigned_four_bytes(input, Endian::Le).unwrap();
        let (input, _sig) = nom_unsigned_four_bytes(input, Endian::Le).unwrap();
        let (input, _crc) = nom_unsigned_four_bytes(input, Endian::Le).unwrap();
        let _ = decompress_rtf(input, &uncompressed_size).unwrap();
    }
}
