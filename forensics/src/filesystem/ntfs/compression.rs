use super::sector_reader::SectorReader;
use crate::filesystem::error::FileSystemError;
use crate::utils::compression::decompress::decompress_xpress;
use crate::{
    filesystem::ntfs::{attributes::get_attribute_data, raw_files::raw_read_data},
    utils::{
        compression::decompress::XpressType,
        nom_helper::{Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes},
    },
};
use log::{error, warn};
use nom::bytes::complete::take;
use ntfs::NtfsFile;
use ntfs::{Ntfs, NtfsAttributeType, NtfsError, structured_values::NtfsAttributeList};
use std::{fs::File, io::BufReader};

#[cfg(target_os = "windows")]
use crate::utils::compression::xpress::api::decompress_huffman_api;

/**
 * Check if the `NTFS` file is compressed using `Windows Overlay Filter` (WOF)
 * This may be enabled starting on Windows 10+
 * The actual file data is compressed in the Alternative Data Stream (ADS) `WofCompressedData`
 * We need to decompress the data in order to get the actual file contents
 */
pub(crate) fn check_wofcompressed(
    ntfs_file: &NtfsFile<'_>,
    ntfs: &Ntfs,
    fs: &mut BufReader<SectorReader<File>>,
) -> Result<(bool, Vec<u8>, u64), NtfsError> {
    let ads = "WofCompressedData";
    let compressed_data = get_attribute_data(ntfs_file, ntfs, fs, ads)?;

    // Skipping files that have compressed data larger than 2GB
    let max_size = 2147483648;
    if compressed_data.len() >= max_size {
        warn!("[wofcompression] Compressed data is larger than 2GB. Skipping decompression");
        let size = compressed_data.len();
        return Ok((true, compressed_data, size as u64));
    }

    // If vec is empty then we did not find WofCompressedData ADS
    if compressed_data.is_empty() {
        return Ok((false, Vec::new(), 0));
    }
    let mut uncompressed_data = Vec::new();

    /*
     * We now have the compressed data, however before we can decompress it we need to figure out the compression method used.
     * We can find out by parsing the `ReparsePoint` attribute  type from the file. This binary data will contain the compression algorithm (unit)
     */
    let compression_unit = grab_reparsepoint(ntfs_file, ntfs, fs)?;
    let lzx32k = 32768;
    if compression_unit == lzx32k {
        warn!("[wofcompression] Lzx compression is not supported! Returning compressed data");
        let size = compressed_data.len();
        return Ok((true, compressed_data, size as u64));
    }

    //let ntfs_file = ntfs_ref.to_file(ntfs, fs)?;
    let data = "";
    if let Some(attr) = ntfs_file.data(fs, data) {
        let item = attr?;
        let data_attr = item.to_attribute()?;
        let uncompressed_size = data_attr.value_length();

        /*
         * We now have: compressed data, uncompressed data size, and the compression unit
         * The compressed data is actually composed of two (2) parts:
         *   1. Offset table array. Each array entry is 4 bytes in size and points to an offset that contains our compressed data
         *      a. If the uncompressed file is greater or equal to 4GBs then the array entry is 8 bytes
         *   2. After the table array is the compressed data
         * The offset pointer is relative to the END of the offset table
         *
         * We can determine the table array size by dividing the uncompressed data size by the compression unit
         * We then need to parse the array table and assemble the compressed data
         */

        let array_len = uncompressed_size / compression_unit as u64;
        let compressed_results = walk_offset_table(
            &compressed_data,
            array_len,
            compression_unit,
            uncompressed_size as usize,
        );
        uncompressed_data = if let Ok((_, result)) = compressed_results {
            result
        } else {
            error!("[wofcompression] Could not get real compressed data for decompression");
            let entry_size = 4;
            return Err(NtfsError::BufferTooSmall {
                expected: (array_len * entry_size) as usize,
                actual: compressed_data.len(),
            });
        };
    }

    Ok((true, uncompressed_data, compressed_data.len() as u64))
}

/// Get the compressed data and determine compression unit
fn grab_reparsepoint(
    ntfs_file: &NtfsFile<'_>,
    ntfs: &Ntfs,
    fs: &mut BufReader<SectorReader<File>>,
) -> Result<u32, NtfsError> {
    //let ntfs_file = ntfs_ref.to_file(ntfs, fs)?;

    let attr_raw = ntfs_file.attributes_raw();
    let mut reparse_data: Vec<u8> = Vec::new();

    // Loop through the raw attributes looking for reparsepoint type
    for attrs in attr_raw {
        let attr = attrs?;

        /*
         * If there are a lot of attributes or attributes take up alot of space
         * `NTFS` will create a new MFT record and create an `AttributeList` to track all the attributes for the file
         */
        if attr.ty()? == NtfsAttributeType::AttributeList {
            let list = attr.structured_value::<_, NtfsAttributeList<'_, '_>>(fs)?;
            let mut list_iter = list.entries();
            // Walk the attributelist
            while let Some(entry) = list_iter.next(fs) {
                let entry = entry?;
                // Only care about reparse point
                if entry.ty()? != NtfsAttributeType::ReparsePoint {
                    continue;
                }
                let temp_file = entry.to_file(ntfs, fs)?;

                let entry_attr = entry.to_attribute(&temp_file)?;
                let mut value = entry_attr.value(fs)?;
                reparse_data = raw_read_data(&mut value, fs)?;
                break;
            }
        } else if attr.ty()? == NtfsAttributeType::ReparsePoint {
            let mut value = attr.value(fs)?;
            reparse_data = raw_read_data(&mut value, fs)?;
            break;
        }
    }

    let reparse_result = parse_reparse(&reparse_data);
    let (_, reparse) = if let Ok(result) = reparse_result {
        result
    } else {
        error!(
            "[wofcompression] Could not parse reparse data, will not be able to decompress data"
        );
        return Err(NtfsError::BufferTooSmall {
            expected: 16,
            actual: reparse_data.len(),
        });
    };

    let lzxpress_huffman4k = 4096;
    let lzx32k = 32768;
    let lzxpress_huffman8k = 8192;
    let lzxpress_huffman16k = 16384;

    let unit = match reparse.compression_method {
        0 => lzxpress_huffman4k,
        1 => lzx32k,
        2 => lzxpress_huffman8k,
        3 => lzxpress_huffman16k,
        _ => {
            error!(
                "[wofcompression] Unknown compression unit {}, will not be able to decompress data",
                reparse.compression_method
            );
            return Err(NtfsError::InvalidStructuredValueSize {
                position: ntfs_file.position(),
                ty: NtfsAttributeType::ReparsePoint,
                expected: 0,
                actual: reparse.compression_method as u64,
            });
        }
    };

    Ok(unit)
}

struct WofReparse {
    _sig: u32,
    _size: u32,
    _wof_version: u32,
    _wof_provider: u32,
    _file_info_version: u32,
    compression_method: u32,
}

/// Parse the reparse attribute type. Its 16 bytes and contains the compression unit used to compress `WofData`
fn parse_reparse(data: &[u8]) -> nom::IResult<&[u8], WofReparse> {
    let (input, sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, wof_version) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, wof_provider) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, file_info_version) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, compression_method) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let reparse = WofReparse {
        _sig: sig,
        _size: size,
        _wof_version: wof_version,
        _wof_provider: wof_provider,
        _file_info_version: file_info_version,
        compression_method,
    };

    Ok((input, reparse))
}

/// Parse the compressed data by first walking the offset table and then decompressing each data chunk
fn walk_offset_table(
    data: &[u8],
    array_len: u64,
    compression_unit: u32,
    uncompressed_size: usize,
) -> nom::IResult<&[u8], Vec<u8>> {
    let mut array_count = 0;
    let mut input = data;

    let mut array_offset = Vec::new();
    // Grab all offsets
    while array_count < array_len {
        let large_uncompressed = 4294967296; // 4GBs
        let (remaining_input, offset) = if uncompressed_size < large_uncompressed {
            let (data, result) = nom_unsigned_four_bytes(input, Endian::Le)?;
            (data, result as u64)
        } else {
            nom_unsigned_eight_bytes(input, Endian::Le)?
        };

        /*
         * If the offset is larger than total data, we have gone outside our array
         * This is possible if we have files that can be evenly divided by the compression unit
         *
         * Ex: C:\Program Files\Windows Defender Advanced Threat Protection\SenseCncPS.dll is compressed with huffman8k (8192)
         * The decompressed size is 16,384 (16,384/8192 = array length of two (2))
         * However the actual length is one (1)
         */
        if offset as usize > data.len() {
            break;
        }
        array_offset.push(offset);

        let increment = 1;
        array_count += increment;
        input = remaining_input;
    }

    /*
     * Now have all offsets
     * Need to assemble compressed data chunks
     * First chunk is actually right after the offset table
     * Reminder: All compressed chunks are after the offset table
     */
    let mut first_chunk = true;
    let mut offset_iter = array_offset.iter().peekable();
    let mut uncompressed_data = Vec::new();
    let mut decom_size = compression_unit;

    while let Some(offset) = offset_iter.next() {
        let (offset_start, first_chunk_data) = take(offset.to_owned())(input)?;
        let mut compressed_data;

        if let Some(next_offset) = offset_iter.peek() {
            let offse_size = *next_offset - offset;
            let (_, compressed_input) = take(offse_size)(offset_start)?;
            compressed_data = compressed_input.to_vec();
        } else {
            // If there is only one array entry then always make sure the first chunk is read (first chunk is NOT part of the array of offsets)
            if first_chunk && !first_chunk_data.is_empty() {
                let uncompressed_result =
                    decompress_ntfs(&mut first_chunk_data.to_vec(), decom_size);
                let mut uncompressed = match uncompressed_result {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[wofcompression] Could not decompress chunk: {err:?}");
                        return Err(nom::Err::Incomplete(nom::Needed::Unknown));
                    }
                };
                first_chunk = false;
                uncompressed_data.append(&mut uncompressed);
            }

            // The last offset entry uses the rest of the data
            compressed_data = offset_start.to_vec();

            // The decompress size is the remaining bytes we need to get the actual decompressed file size
            // The decompressed file size was obtained by getting the data length of the sparse data attribute
            let last_size = (uncompressed_size - uncompressed_data.len()).try_into();
            decom_size = match last_size {
                Ok(result) => result,
                Err(err) => {
                    error!("[wofcompression] Could not get last offset size: {err:?}");
                    return Err(nom::Err::Incomplete(nom::Needed::Unknown));
                }
            };
        }

        if first_chunk && !first_chunk_data.is_empty() {
            let uncompressed_result = decompress_ntfs(&mut first_chunk_data.to_vec(), decom_size);
            let mut uncompressed = match uncompressed_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[wofcompression] Could not decompress chunk: {err:?}");
                    return Err(nom::Err::Incomplete(nom::Needed::Unknown));
                }
            };
            first_chunk = false;
            uncompressed_data.append(&mut uncompressed);
        }

        // If the compressed data chunk equals the compression unit, then its already uncompressed...
        if compressed_data.len() == decom_size as usize {
            uncompressed_data.append(&mut compressed_data);
            continue;
        }

        let uncompressed_result = decompress_ntfs(&mut compressed_data, decom_size);
        let mut uncompressed = match uncompressed_result {
            Ok(result) => result,
            Err(err) => {
                error!("[wofcompression] Could not decompress chunk: {err:?}");
                return Err(nom::Err::Incomplete(nom::Needed::Unknown));
            }
        };
        uncompressed_data.append(&mut uncompressed);
    }

    Ok((input, uncompressed_data))
}

#[cfg(target_os = "windows")]
/// Decompress WOF compressed data on Windows systems
fn decompress_ntfs(data: &mut [u8], decom_size: u32) -> Result<Vec<u8>, FileSystemError> {
    let pf_data_result = decompress_huffman_api(data, &XpressType::XpressHuffman, decom_size);
    let pf_data = match pf_data_result {
        Ok(result) => result,
        Err(err) => {
            error!(
                "[wofcompression] Could not decompress data: {err:?}. Will try manual decompression"
            );
            let pf_data_result = decompress_xpress(data, decom_size, &XpressType::XpressHuffman);
            match pf_data_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[wofcompression] Could not decompress data: {err:?}");
                    return Err(FileSystemError::FileData);
                }
            }
        }
    };

    Ok(pf_data)
}

#[cfg(target_family = "unix")]
/// Decompress WOF compressed data on non-Windows systems
fn decompress_ntfs(data: &mut [u8], decom_size: u32) -> Result<Vec<u8>, FileSystemError> {
    let ntfs_data_result = decompress_xpress(data, decom_size, &XpressType::XpressHuffman);
    let ntfs_data = match ntfs_data_result {
        Ok(result) => result,
        Err(err) => {
            error!("[wofcompression] Could not decompress data: {err:?}");
            return Err(FileSystemError::FileData);
        }
    };

    Ok(ntfs_data)
}

#[cfg(test)]
#[cfg(target_os = "windows")]
mod tests {
    use super::{grab_reparsepoint, parse_reparse, walk_offset_table};
    use crate::{
        filesystem::ntfs::{
            compression::check_wofcompressed,
            raw_files::{NtfsOptions, get_user_registry_files, iterate_ntfs},
            setup::setup_ntfs_parser,
        },
        utils::regex_options::create_regex,
    };

    #[test]
    fn test_parse_reparse() {
        let test_data = [
            23, 0, 0, 128, 16, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0,
        ];
        let (_, result) = parse_reparse(&test_data).unwrap();
        assert_eq!(result._sig, 2147483671);
        assert_eq!(result._size, 16);
        assert_eq!(result._wof_version, 1);
        assert_eq!(result._wof_provider, 2);
        assert_eq!(result._file_info_version, 1);
        assert_eq!(result.compression_method, 2);
    }

    #[test]
    fn test_walk_offset_table() {
        let test_data = [
            0, 0, 0, 0, 149, 136, 152, 154, 167, 168, 153, 170, 183, 138, 154, 154, 8, 11, 9, 144,
            151, 11, 7, 171, 8, 11, 10, 152, 183, 139, 154, 154, 136, 170, 185, 187, 135, 154, 103,
            187, 134, 11, 119, 170, 183, 170, 137, 169, 8, 10, 137, 153, 151, 154, 153, 154, 136,
            187, 169, 153, 7, 152, 120, 155, 183, 186, 138, 170, 135, 144, 104, 169, 87, 107, 121,
            176, 7, 0, 176, 176, 7, 176, 170, 186, 167, 160, 10, 160, 184, 176, 10, 160, 8, 0, 11,
            160, 183, 169, 187, 187, 151, 169, 8, 121, 151, 169, 168, 170, 168, 171, 11, 170, 184,
            185, 11, 160, 183, 0, 11, 160, 183, 144, 9, 160, 168, 160, 0, 137, 8, 153, 155, 123,
            11, 0, 11, 0, 0, 0, 0, 0, 176, 0, 0, 0, 0, 0, 0, 0, 185, 176, 160, 0, 176, 0, 0, 176,
            150, 89, 150, 112, 0, 0, 0, 176, 151, 168, 170, 0, 0, 11, 0, 0, 166, 137, 10, 11, 187,
            160, 0, 186, 135, 105, 184, 11, 9, 0, 176, 144, 118, 137, 169, 187, 11, 0, 187, 144,
            119, 120, 185, 186, 0, 171, 0, 160, 103, 135, 136, 185, 10, 0, 186, 171, 103, 137, 153,
            171, 186, 11, 0, 112, 121, 168, 9, 10, 11, 0, 0, 0, 112, 152, 168, 169, 0, 11, 11, 138,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 209, 85, 4, 51,
            0, 168, 109, 9, 241, 144, 0, 224, 161, 216, 105, 248, 31, 203, 177, 151, 188, 125, 10,
            51, 142, 200, 75, 231, 131, 253, 241, 53, 198, 17, 182, 113, 79, 201, 141, 82, 210, 18,
            232, 166, 45, 231, 207, 179, 41, 42, 29, 206, 76, 61, 99, 207, 158, 164, 211, 243, 232,
            41, 105, 103, 151, 82, 146, 146, 231, 231, 40, 4, 186, 46, 157, 53, 127, 155, 155, 98,
            196, 210, 37, 178, 219, 162, 223, 124, 214, 174, 218, 141, 108, 136, 127, 201, 50, 186,
            114, 31, 253, 70, 52, 154, 189, 157, 186, 203, 135, 12, 125, 232, 213, 29, 246, 220,
            86, 90, 21, 135, 252, 219, 186, 87, 69, 12, 141, 22, 118, 116, 91, 216, 48, 59, 118,
            47, 116, 145, 215, 29, 180, 99, 49, 216, 170, 131, 109, 165, 159, 210, 204, 57, 91,
            120, 111, 140, 21, 102, 0, 104, 180, 139, 128, 235, 181, 222, 117, 88, 58, 219, 131,
            144, 3, 228, 148, 64, 121, 92, 115, 1, 184, 99, 216, 9, 64, 1, 64, 183, 55, 0, 151,
            237, 17, 159, 196, 248, 9, 156, 189, 15, 125, 30, 152, 105, 33, 57, 201, 147, 219, 255,
            222, 206, 81, 80, 130, 244, 13, 2, 192, 87, 35, 248, 117, 216, 255, 185, 45, 138, 150,
            199, 143, 130, 178, 111, 77, 5, 50, 200, 246, 207, 216, 133, 1, 192, 116, 99, 1, 6,
            144, 193, 71, 151, 127, 168, 96, 126, 14, 129, 177, 168, 110, 31, 25, 0, 130, 198, 44,
            144, 251, 255, 100, 54, 78, 132, 173, 56, 182, 69, 19, 205, 90, 216, 116, 119, 38, 124,
            106, 111, 90, 199, 161, 108, 1, 86, 22, 110, 154, 11, 100, 222, 48, 126, 42, 110, 74,
            54, 159, 157, 190, 210, 240, 157, 206, 1, 210, 31, 24, 191, 37, 128, 80, 143, 4, 155,
            247, 233, 165, 180, 105, 191, 20, 22, 16, 172, 150, 121, 198, 218, 67, 0, 102, 121, 3,
            108, 186, 98, 214, 48, 159, 117, 0, 206, 24, 203, 64, 132, 24, 15, 133, 229, 121, 56,
            65, 158, 0, 212, 110, 150, 224, 51, 165, 50, 108, 7, 0, 60, 144, 67, 177, 193, 114,
            111, 136, 182, 54, 55, 213, 144, 33, 1, 69, 117, 98, 143, 223, 161, 229, 20, 250, 36,
            166, 175, 0, 238, 132, 120, 234, 51, 23, 49, 189, 247, 67, 77, 33, 166, 160, 190, 204,
            108, 72, 3, 134, 130, 185, 85, 217, 161, 229, 221, 60, 111, 54, 159, 59, 48, 49, 1,
            157, 240, 5, 249, 28, 231, 149, 110, 213, 58, 78, 1, 40, 126, 206, 252, 113, 187, 214,
            14, 240, 169, 170, 28, 148, 84, 78, 57, 127, 134, 63, 39, 29, 148, 117, 150, 1, 43,
            117, 234, 32, 172, 142, 185, 23, 78, 14, 174, 3, 64, 86, 206, 17, 142, 142, 150, 20,
            73, 150, 79, 92, 115, 88, 85, 58, 57, 192, 183, 240, 173, 39, 41, 190, 235, 99, 21,
            181, 232, 31, 47, 239, 16, 67, 137, 164, 44, 1, 165, 99, 54, 26, 184, 20, 146, 66, 201,
            242, 136, 228, 16, 28, 163, 171, 143, 202, 79, 78, 142, 119, 152, 250, 141, 238, 73,
            156, 99, 100, 93, 148, 161, 209, 176, 32, 43, 75, 214, 203, 142, 244, 19, 84, 185, 24,
            192, 59, 157, 143, 241, 41, 187, 143, 101, 127, 251, 48, 21, 4, 20, 199, 97, 10, 13,
            174, 112, 164, 82, 42, 144, 189, 33, 36, 62, 103, 208, 121, 231, 46, 128, 210, 178,
            247, 158, 211, 149, 37, 235, 133, 246, 58, 81, 116, 130, 26, 225, 248, 117, 176, 115,
            4, 32, 133, 100, 208, 52, 4, 19, 193, 100, 145, 93, 100, 56, 1, 212, 229, 150, 141,
            226, 164, 47, 158, 159, 19, 230, 215, 174, 101, 40, 222, 135, 82, 13, 10, 203, 118,
            155, 84, 55, 230, 103, 126, 173, 64, 141, 76, 85, 118, 133, 201, 47, 59, 218, 220, 93,
            56, 6, 22, 14, 196, 223, 73, 70, 190, 33, 148, 88, 109, 230, 0, 122, 195, 174, 242,
            191, 115, 207, 232, 230, 102, 23, 6, 25, 89, 215, 160, 209, 148, 11, 26, 174, 160, 191,
            214, 195, 6, 15, 114, 123, 243, 92, 202, 195, 61, 247, 28, 137, 65, 3, 76, 3, 56, 172,
            71, 217, 41, 83, 196, 8, 169, 212, 205, 133, 6, 87, 200, 29, 87, 249, 21, 173, 205,
            132, 142, 189, 167, 86, 25, 6, 144, 102, 96, 122, 179, 240, 187, 41, 86, 52, 93, 161,
            25, 217, 159, 230, 102, 196, 119, 32, 104, 154, 104, 33, 89, 151, 234, 74, 151, 181,
            112, 33, 144, 142, 198, 65, 212, 156, 21, 142, 98, 9, 187, 15, 225, 199, 63, 88, 103,
            11, 16, 226, 175, 8, 43, 46, 146, 29, 169, 132, 58, 142, 73, 193, 2, 46, 141, 169, 61,
            1, 144, 191, 167, 57, 18, 10, 229, 164, 32, 12, 105, 87, 42, 7, 138, 47, 40, 68, 149,
            221, 37, 84, 68, 162, 157, 251, 250, 229, 71, 132, 84, 108, 121, 180, 231, 202, 204,
            100, 149, 75, 180, 58, 96, 174, 169, 121, 61, 173, 63, 137, 159, 150, 229, 202, 212,
            242, 181, 125, 174, 219, 247, 184, 205, 100, 238, 9, 76, 109, 51, 71, 1, 88, 194, 9,
            186, 246, 199, 59, 44, 55, 35, 57, 51, 117, 96, 159, 56, 238, 39, 165, 198, 184, 169,
            208, 187, 39, 220, 138, 54, 112, 219, 50, 12, 5, 82, 178, 225, 159, 139, 149, 202, 138,
            30, 188, 101, 196, 115, 120, 178, 7, 191, 156, 63, 51, 121, 17, 136, 134, 228, 58, 71,
            131, 250, 49, 244, 239, 69, 127, 15, 134, 243, 200, 94, 188, 203, 28, 13, 147, 175,
            136, 121, 125, 44, 246, 36, 224, 47, 154, 69, 139, 170, 195, 53, 173, 63, 148, 137,
            144, 29, 7, 206, 15, 13, 18, 129, 224, 89, 156, 239, 134, 38, 111, 156, 79, 64, 9, 121,
            142, 19, 37, 59, 58, 1, 209, 160, 110, 248, 42, 231, 75, 46, 104, 57, 153, 69, 78, 126,
            99, 25, 64, 120, 53, 117, 130, 176, 169, 191, 117, 53, 17, 116, 144, 214, 122, 199, 55,
            210, 181, 195, 88, 73, 21, 90, 90, 82, 193, 243, 70, 48, 108, 33, 164, 191, 106, 114,
            185, 146, 61, 180, 251, 94, 169, 118, 225, 46, 86, 120, 141, 240, 96, 97, 94, 82, 136,
            128, 35, 135, 79, 83, 31, 138, 229, 68, 0, 226, 137, 42, 105, 233, 218, 245, 87, 230,
            188, 107, 236, 56, 13, 177, 52, 185, 210, 192, 189, 192, 44, 20, 84, 56, 86, 165, 154,
            28, 47, 90, 185, 50, 14, 213, 2, 153, 21, 190, 42, 254, 3, 14, 119, 149, 211, 71, 0,
            162, 233, 228, 15, 46, 3, 68, 237, 17, 181, 14, 90, 83, 192, 208, 190, 249, 30, 241,
            100, 95, 108, 234, 41, 110, 245, 246, 202, 91, 83, 113, 73, 53, 62, 91, 241, 119, 7,
            80, 96, 37, 227, 52, 19, 120, 31, 235, 139, 22, 151, 52, 11, 58, 139, 195, 115, 236,
            203, 64, 86, 37, 114, 249, 42, 160, 128, 181, 5, 18, 130, 50, 184, 8, 247, 29, 25, 191,
            193, 17, 196, 212, 156, 117, 9, 19, 18, 112, 15, 219, 17, 224, 83, 60, 118, 199, 139,
            112, 227, 103, 217, 43, 48, 47, 216, 64, 207, 195, 188, 27, 114, 163, 176, 91, 104, 99,
            33, 98, 42, 35, 68, 67, 13, 33, 190, 224, 129, 232, 187, 49, 134, 242, 244, 93, 67,
            119, 27, 178, 223, 179, 80, 201, 162, 248, 49, 100, 218, 80, 195, 134, 74, 93, 178, 70,
            120, 44, 142, 5, 102, 251, 166, 29, 166, 221, 140, 85, 228, 28, 84, 185, 37, 119, 6,
            48, 191, 249, 73, 106, 80, 132, 25, 210, 95, 71, 223, 162, 121, 20, 126, 223, 92, 113,
            168, 47, 208, 67, 225, 141, 21, 123, 65, 27, 104, 42, 103, 80, 42, 253, 28, 104, 157,
            100, 62, 64, 175, 80, 178, 65, 55, 40, 202, 133, 174, 239, 145, 233, 107, 127, 79, 205,
            231, 37, 93, 222, 50, 109, 1, 8, 157, 187, 236, 171, 36, 47, 118, 65, 30, 103, 65, 87,
            195, 80, 16, 60, 101, 36, 196, 126, 48, 97, 124, 97, 133, 71, 217, 143, 101, 70, 166,
            3, 152, 133, 132, 150, 175, 54, 39, 114, 53, 207, 249, 132, 228, 158, 133, 47, 66, 197,
            106, 139, 118, 102, 17, 183, 66, 203, 202, 137, 160, 32, 216, 96, 123, 66, 2, 78, 115,
            44, 78, 90, 152, 103, 119, 16, 156, 173, 188, 172, 13, 66, 138, 53, 156, 40, 226, 124,
            139, 104, 5, 203, 129, 184, 195, 116, 34, 130, 182, 233, 209, 120, 182, 254, 180, 225,
            164, 41, 199, 80, 39, 148, 160, 36, 19, 226, 198, 220, 140, 133, 102, 44, 128, 90, 144,
            129, 199, 145, 204, 1, 51, 72, 232, 251, 72, 5, 20, 214, 148, 15, 203, 203, 89, 171,
            236, 19, 128, 250, 54, 1, 156, 215, 19, 102, 102, 79, 248, 244, 158, 51, 90, 15, 133,
            111, 133, 94, 109, 14, 159, 238, 254, 132, 123, 247, 61, 44, 183, 102, 255, 129, 252,
            1, 27, 150, 64, 166, 53, 107, 26, 18, 151, 224, 44, 169, 228, 13, 9, 101, 205, 225,
            206, 239, 154, 203, 218, 211, 109, 38, 54, 26, 212, 150, 234, 210, 5, 247, 70, 246,
            231, 151, 187, 190, 6, 52, 19, 128, 102, 112, 211, 87, 7, 1, 186, 233, 209, 25, 135,
            72, 139, 91, 2, 54, 5, 203, 197, 171, 139, 149, 22, 57, 44, 78, 89, 220, 179, 48, 100,
            97, 213, 130, 200, 130, 170, 197, 128, 197, 212, 44, 198, 90, 176, 46, 130, 134, 133,
            206, 139, 153, 139, 87, 22, 49, 44, 110, 90, 192, 89, 176, 179, 120, 101, 81, 204, 162,
            169, 197, 137, 33, 134, 48, 198, 36, 75, 228, 12, 73, 15, 74, 253, 178, 233, 244, 145,
            78, 162, 235, 199, 122, 231, 122, 185, 94, 174, 167, 235, 101, 245, 68, 191, 217, 238,
            246, 237, 172, 176, 235, 220, 122, 109, 189, 186, 158, 173, 183, 111, 161, 247, 207,
            188, 89, 19, 192, 203, 25, 148, 144, 49, 186, 28, 38, 205, 226, 202, 153, 30, 55, 132,
            249, 137, 88, 236, 85, 232, 187, 23, 170, 196, 117, 146, 24, 18, 153, 113, 108, 189,
            80, 214, 223, 69, 185, 43, 25, 61, 134, 92, 38, 16, 66, 158, 19, 205, 33, 77, 137, 74,
            41, 223, 52, 81, 19, 114, 156, 8, 100, 131, 78, 144, 29, 249, 186, 9, 131, 144, 239,
            196, 120, 144, 106, 156, 128, 144, 223, 9, 228, 29, 32, 2, 214, 140, 175, 65, 103, 19,
            215, 32, 176, 137, 186, 41, 62, 218, 220, 5, 129, 69, 99, 113, 175, 232, 122, 46, 17,
            198, 105, 108, 52, 244, 227, 43, 255, 108, 95, 234, 194, 199, 163, 18, 228, 233, 248,
            60, 195, 114, 22, 81, 44, 7, 150, 78, 139, 38, 37, 19, 69, 132, 229, 139, 252, 238,
            126, 7, 88, 48, 28, 22, 27, 150, 42, 22, 13, 203, 12, 11, 20, 68, 95, 37, 52, 181, 83,
            155, 6, 187, 51, 239, 138, 78, 179, 9, 16, 38, 70, 35, 52, 180, 127, 156, 105, 16, 26,
            194, 173, 169, 178, 103, 255, 242, 253, 244, 127, 215, 33, 185, 236, 213, 146, 144, 72,
            246, 244, 210, 27, 252, 132, 164, 171, 119, 109, 107, 50, 180, 251, 199, 116, 137, 73,
            200, 225, 0, 21, 73, 139, 135, 15, 36, 44, 146, 194, 79, 201, 94, 118, 21, 171, 139,
            173, 224, 160, 136, 161, 151, 56, 38, 206, 34, 167, 237, 73, 118, 185, 142, 186, 53,
            15, 87, 190, 119, 221, 93, 154, 221, 37, 95, 93, 199, 29, 161, 92, 42, 198, 4, 171,
            195, 101, 13, 172, 137, 53, 183, 49, 151, 38, 215, 150, 6, 42, 210, 212, 227, 234, 75,
            174, 1, 205, 31, 160, 153, 142, 52, 45, 185, 31, 100, 134, 226, 182, 32, 205, 193, 178,
            130, 101, 197, 202, 133, 168, 139, 147, 22, 43, 44, 76, 178, 56, 100, 65, 200, 226,
            167, 197, 128, 69, 152, 44, 198, 90, 30, 15, 112, 255, 161, 67, 158, 100, 87, 46, 76,
            139, 55, 139, 70, 22, 88, 45, 136, 44, 168, 89, 5, 176, 139, 153, 161, 89, 132, 225,
            16, 134, 152, 196, 92, 199, 119, 172, 196, 99, 153, 78, 174, 207, 204, 31, 92, 252, 94,
            157, 104, 156, 86, 46, 104, 2, 85, 46, 104, 226, 78, 46, 208, 68, 155, 92, 160, 137,
            54, 185, 65, 19, 172, 114, 65, 19, 171, 114, 65, 19, 106, 114, 130, 38, 208, 228, 5,
            77, 152, 201, 11, 154, 17, 147, 23, 52, 2, 38, 46, 104, 196, 75, 68, 216, 173, 14, 171,
            224, 68, 82, 144, 222, 130, 184, 176, 228, 29, 185, 155, 114, 10, 185, 142, 89, 120,
            136, 72, 228, 72, 46, 43, 248, 181, 151, 189, 150, 162, 92, 106, 212, 79, 107, 122,
            130, 146, 139, 133, 11, 44, 249, 39, 101, 157, 239, 119, 226, 206, 151, 191, 241, 150,
            20, 246, 253, 132, 135, 86, 186, 63, 93, 96, 27, 241, 18, 109, 13, 87, 3, 26, 5, 215,
            179, 18, 38, 237, 25, 130, 147, 31, 184, 101, 29, 148, 148, 105, 140, 124, 176, 73,
            210, 146, 172, 241, 26, 178, 67, 123, 19, 56, 65, 204, 77, 207, 196, 49, 16, 183, 217,
            85, 4, 36, 153, 198, 86, 33, 56, 16, 167, 70, 19, 88, 110, 89, 237, 227, 40, 11, 85,
            102, 239, 169, 97, 20, 11, 65, 91, 87, 130, 73, 152, 11, 216, 100, 120, 5, 3, 194, 88,
            54, 172, 249, 108, 226, 188, 18, 112, 95, 39, 245, 216, 222, 170, 255, 40, 149, 224,
            60, 229, 28, 1, 172, 34, 178, 37, 133, 240, 175, 241, 74, 190, 97, 25, 16, 31, 181,
            162, 3, 232, 87, 130, 205, 138, 27, 249, 4, 230, 216, 130, 131, 157, 87, 28, 162, 164,
            243, 99, 205, 9, 251, 28, 132, 100, 130, 121, 137, 87, 36, 20, 141, 220, 252, 192, 213,
            18, 204, 72, 188, 23, 48, 100, 215, 1, 175, 0, 119, 38, 168, 6, 121, 74, 25, 97, 127,
            115, 97, 198, 87, 130, 56, 147, 0, 121, 37, 252, 215, 16, 71, 52, 162, 211, 56, 192, 2,
            99, 23, 184, 228, 84, 215, 71, 107, 170, 148, 79, 229, 79, 242, 231, 133, 222, 186, 74,
            61, 185, 34, 248, 92, 120, 7, 62, 234, 99, 71, 250, 11, 154, 56, 186, 64, 210, 169,
            187, 71, 189, 129, 228, 132, 142, 223, 112, 140, 129, 65, 144, 176, 238, 7, 1, 96, 234,
            15, 2, 80, 102, 76, 232, 204, 7, 115, 40, 15, 197, 80, 144, 242, 247, 145, 15, 70, 215,
            187, 247, 113, 51, 214, 107, 188, 64, 89, 132, 214, 152, 83, 87, 191, 177, 42, 240,
            153, 68, 187, 56, 118, 149, 142, 133, 177, 196, 139, 242, 80, 29, 140, 130, 180, 3,
            223, 249, 7, 10, 79, 133, 59, 7, 124, 175, 143, 243, 245, 0, 176, 53, 81, 39, 2, 251,
            124, 87, 61, 107, 243, 48, 149, 58, 220, 153, 134, 143, 255, 240, 35, 174, 242, 247,
            22, 133, 167, 120, 201, 233, 73, 5, 13, 45, 118, 126, 69, 132, 147, 24, 206, 221, 28,
            60, 168, 43, 157, 164, 134, 234, 238, 92, 128, 18, 12, 155, 74, 3, 52, 227, 71, 22,
            255, 156, 5, 237, 128, 205, 206, 249, 215, 155, 70, 99, 254, 88, 51, 143, 26, 17, 20,
            26, 90, 242, 53, 24, 109, 10, 141, 193, 233, 33, 120, 135, 60, 221, 70, 238, 117, 81,
            174, 148, 89, 163, 160, 133, 189, 39, 80, 39, 246, 27, 60, 120, 112, 208, 156, 164, 7,
            102, 76, 161, 142, 26, 60, 51, 131, 91, 5, 189, 30, 244, 8, 163, 99, 16, 9, 93, 1, 232,
            27, 191, 81, 100, 238, 24, 53, 39, 191, 174, 91, 72, 209, 6, 247, 132, 227, 78, 104,
            247, 142, 182, 1, 132, 195, 168, 158, 193, 92, 245, 21, 7, 247, 223, 201, 64, 112, 103,
            6, 9, 26, 187, 251, 245, 59, 161, 157, 26, 116, 51, 142, 239, 59, 127, 1, 152, 39, 237,
            9, 34, 37, 63, 246, 28, 123, 234, 126, 96, 42, 31, 35, 118, 186, 205, 104, 5, 212, 144,
            83, 87, 212, 4, 91, 102, 224, 37, 220, 122, 59, 75, 46, 6, 58, 59, 232, 219, 247, 99,
            6, 86, 212, 138, 28, 53, 118, 102, 11, 222, 206, 146, 97, 208, 224, 101, 247, 194, 225,
            48, 153, 123, 80, 19, 94, 200, 148, 110, 219, 147, 39, 62, 12, 56, 120, 212, 185, 189,
            241, 114, 66, 193, 135, 195, 3, 59, 157, 37, 91, 181, 108, 247, 251, 193, 140, 170,
            118, 22, 153, 251, 11, 172, 200, 135, 37, 177, 176, 24, 59, 34, 140, 180, 122, 184,
            173, 236, 172, 155, 122, 237, 252, 38, 253, 187, 135, 10, 144, 91, 150, 192, 198, 75,
            153, 132, 8, 94, 228, 202, 217, 202, 89, 49, 154, 211, 216, 227, 247, 243, 51, 234,
            240, 26, 26, 82, 45, 26, 39, 240, 31, 155, 202, 236, 123, 126, 121, 98, 79, 47, 217,
            253, 37, 191, 122, 100, 209, 79, 40, 198, 107, 227, 99, 127, 210, 238, 107, 28, 26,
            192, 38, 133, 96, 123, 196, 207, 96, 108, 122, 202, 164, 59, 22, 227, 189, 206, 96,
            164, 199, 22, 253, 29, 240, 195, 210, 99, 78, 105, 28, 236, 123, 85, 72, 102, 227, 39,
            61, 182, 232, 36, 139, 95, 147, 30, 244, 175, 23, 35, 27, 222, 147, 77, 122, 33, 149,
            11, 130, 25, 92, 146, 83, 5, 2, 26, 113, 123, 161, 194, 211, 44, 9, 133, 76, 82, 0, 0,
            237, 10, 238, 32, 37, 20, 44, 95, 125, 0, 0, 240, 0, 0,
        ];
        let length = 1;
        let unit = 8192;
        let uncompressed_size = 8192;

        let (_, result) = walk_offset_table(&test_data, length, unit, uncompressed_size).unwrap();
        assert_eq!(result.len(), 8192);
    }

    #[test]
    fn test_check_wofcompressed() {
        let result = get_user_registry_files('C').unwrap();

        // Should at least have three (3). User (NTUSER and UsrClass), Default (NTUSER)
        assert!(result.len() >= 3);
        let mut ntfs_parser = setup_ntfs_parser('C').unwrap();
        for entry in result {
            let (is_compressed, uncompressed, compressed_size) = check_wofcompressed(
                &entry
                    .reg_reference
                    .to_file(&ntfs_parser.ntfs, &mut ntfs_parser.fs)
                    .unwrap(),
                &ntfs_parser.ntfs,
                &mut ntfs_parser.fs,
            )
            .unwrap();
            assert_eq!(is_compressed, false);
            assert_eq!(uncompressed.is_empty(), true);
            assert_eq!(compressed_size, 0);
        }
    }

    #[test]
    #[ignore = "Typically requires HDD or explorer.exe to be compressed: 'compact.exe /c /exe:XPRESS4K C:\\Windows\\explorer.exe'"]
    fn test_grab_reparsepoint() {
        let path = "C:\\Windows\\explorer.exe";

        let drive = 'C';
        let mut ntfs_parser = setup_ntfs_parser(drive).unwrap();
        let root_dir = ntfs_parser
            .ntfs
            .root_directory(&mut ntfs_parser.fs)
            .unwrap();

        let mut ntfs_options = NtfsOptions {
            start_path: path.to_string(),
            start_path_depth: 0,
            depth: path.split('\\').count(),
            path_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            file_regex: create_regex("").unwrap(), // Valid Regex, should never fail
            filelist: Vec::new(),
            directory_tracker: vec![format!("{drive}:")],
        };

        // Search and iterate through the NTFS system for the file
        let _ = iterate_ntfs(
            root_dir,
            &mut ntfs_parser.fs,
            &ntfs_parser.ntfs,
            &mut ntfs_options,
        );

        for filelist in ntfs_options.filelist {
            if filelist.full_path != path {
                continue;
            }

            let unit = grab_reparsepoint(
                &filelist
                    .file
                    .to_file(&ntfs_parser.ntfs, &mut ntfs_parser.fs)
                    .unwrap(),
                &ntfs_parser.ntfs,
                &mut ntfs_parser.fs,
            )
            .unwrap();
            let mut is_unit = false;
            if unit == 4096 || unit == 8192 || unit == 32768 || unit == 16384 {
                is_unit = true;
            }
            assert_eq!(is_unit, true);
            break;
        }
    }
}
