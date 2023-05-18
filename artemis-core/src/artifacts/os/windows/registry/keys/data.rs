use crate::{
    artifacts::os::windows::registry::cell::is_allocated,
    utils::{
        encoding::base64_encode_standard,
        nom_helper::{nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian},
        strings::{extract_multiline_utf16_string, extract_utf16_string},
    },
};
use log::error;
use nom::{
    bytes::complete::take,
    number::complete::{le_i64, le_u64},
};

#[derive(PartialEq)]
enum DataTypes {
    MultiString,
    String,
    Binary,
}

/// Parse Registry data that contains a string
pub(crate) fn parse_reg_sz(
    reg_data: &[u8],
    offset: u32,
    data_size: u32,
    minor_version: u32,
) -> nom::IResult<&[u8], String> {
    let data_type = DataTypes::String;
    let (_, value) = check_big_data(reg_data, offset, data_size, data_type, minor_version)?;

    Ok((reg_data, value))
}

/// Parse Registry data that contains a multi-line string
pub(crate) fn parse_reg_multi_sz(
    reg_data: &[u8],
    offset: u32,
    data_size: u32,
    minor_version: u32,
) -> nom::IResult<&[u8], String> {
    let data_type = DataTypes::MultiString;
    let (_, value) = check_big_data(reg_data, offset, data_size, data_type, minor_version)?;

    Ok((reg_data, value))
}

/// Parse Registry data that contains binary data. Return as base64 encoded string
pub(crate) fn parse_reg_binary(
    reg_data: &[u8],
    offset: u32,
    data_size: u32,
    minor_version: u32,
) -> nom::IResult<&[u8], String> {
    let data_type = DataTypes::Binary;
    let (_, value) = check_big_data(reg_data, offset, data_size, data_type, minor_version)?;

    Ok((reg_data, value))
}

/// Parse Registry data that contains a qword (64 bit) data.
pub(crate) fn parse_qword_filetime(
    reg_data: &[u8],
    offset: u32,
    data_size: u32,
    filetime: bool,
) -> nom::IResult<&[u8], String> {
    let (input, _) = take(offset as usize)(reg_data)?;
    let (input, (allocated, data_cell_size)) = is_allocated(input)?;

    if !allocated {
        error!("[registry] Got unallocated FILETIME/QWORD data");
        return Ok((reg_data, String::new()));
    }
    // Size includes the size itself. We nommed that away
    let adjust_cell_size = 4;
    let (_, allocated_data) = take(data_cell_size - adjust_cell_size)(input)?;

    // The size in the Value key is the actual size of the data
    // Any remaining data is slack space
    let (_slack_space, allocated_data) = take(data_size)(allocated_data)?;

    let value = if filetime {
        let (_, value) = le_u64(allocated_data)?;
        format!("{value}")
    } else {
        let (_, value) = le_i64(allocated_data)?;
        format!("{value}")
    };
    Ok((reg_data, value))
}

/// Parse the Registry data while checking for big data signature ("Db")
fn check_big_data(
    reg_data: &[u8],
    offset: u32,
    data_size: u32,
    data_type: DataTypes,
    minor_version: u32,
) -> nom::IResult<&[u8], String> {
    let (input, _) = take(offset as usize)(reg_data)?;

    let (input, (allocated, data_cell_size)) = is_allocated(input)?;

    if !allocated {
        error!("[registry] Got unallocated Big Data");
        return Ok((reg_data, String::new()));
    }
    // The max size of big data is 16344
    let max_size = 16344;
    let min_version = 3;

    // If Value data is very large, then it is stored in a Db cell list
    // Also Db cell list only exists in Registry versions higher than 1.3.
    if data_size > max_size && minor_version > min_version {
        // large_data contains the whole data for the value key. Even possible padding and slack data (for large strings)
        let (_, (sizes, large_data)) = parse_big_data(reg_data, input)?;

        let mut reg_string = String::new();
        let mut start_size = 0;

        /*
        Since large data values are separated accross the Registry each data value may have slack space or padding
        We loop through each size associated with the value to try to extract the string
        */
        let mut binary_vec = Vec::new();
        let mut sizes_iter = sizes.iter().peekable();
        while let Some(db_size) = sizes_iter.next() {
            if start_size > large_data.len() || max_size as usize > large_data.len() {
                break;
            }

            // Check if we are at last big data entry
            // The last big data entry does not have to equal the max size (16344)
            if sizes_iter.peek().is_none() {
                // The final size should be the difference between the data size specified in the value key and our current data
                let final_size = data_size as usize - binary_vec.len();
                // Adjust the size based on final data size
                let allocated_data = if final_size + start_size < large_data.len() {
                    &large_data[start_size..final_size + start_size]
                } else {
                    &large_data[start_size..]
                };

                if data_type == DataTypes::String {
                    reg_string = format!("{reg_string}{}", extract_utf16_string(allocated_data));
                } else if data_type == DataTypes::MultiString {
                    reg_string = format!(
                        "{reg_string}{}",
                        extract_multiline_utf16_string(allocated_data)
                    );
                } else {
                    binary_vec.append(&mut allocated_data.to_vec());
                }
                continue;
            }

            // Adjust the size based on previous data size
            let allocated_data = &large_data[start_size..max_size as usize + start_size];

            // Track previous data size
            start_size += db_size.to_owned() as usize;

            if data_type == DataTypes::String {
                reg_string = format!("{reg_string}{}", extract_utf16_string(allocated_data));
            } else if data_type == DataTypes::MultiString {
                reg_string = format!(
                    "{reg_string}{}",
                    extract_multiline_utf16_string(allocated_data)
                );
            } else {
                binary_vec.append(&mut allocated_data.to_vec());
            }
        }

        if data_type == DataTypes::Binary {
            reg_string = base64_encode_standard(&binary_vec);
        }

        return Ok((reg_data, reg_string));
    }

    // Size includes the size itself. We nommed that away
    let adjust_cell_size = 4;
    let (_, allocated_data) = take(data_cell_size - adjust_cell_size)(input)?;

    // The size in the Value key is the actual size of the data
    // Any remaining data is slack space
    let (_slack_space, allocated_data) = take(data_size)(allocated_data)?;
    let value = if data_type == DataTypes::String {
        extract_utf16_string(allocated_data)
    } else if data_type == DataTypes::MultiString {
        extract_multiline_utf16_string(allocated_data)
    } else {
        base64_encode_standard(allocated_data)
    };
    Ok((reg_data, value))
}

/// Reassemble large Registry data values in a Db list
fn parse_big_data<'a>(
    reg_data: &'a [u8],
    db_data: &'a [u8],
) -> nom::IResult<&'a [u8], (Vec<u32>, Vec<u8>)> {
    let (input, _sig) = nom_unsigned_two_bytes(db_data, Endian::Le)?;
    let (input, number_offsets) = nom_unsigned_two_bytes(input, Endian::Le)?;

    // Offset to a list of offsets that all point to the large registry value data
    let (_, offset) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (offset_list, _) = take(offset)(reg_data)?;
    let (mut remaining_offsets, (allocated, _)) = is_allocated(offset_list)?;

    // If offset_list is not allocated, just return
    if !allocated {
        return Ok((reg_data, (Vec::new(), Vec::new())));
    }

    // Remaining values in the offset_list are offsets to the registry data
    // Store and track all data associated with large value key
    let mut large_data: Vec<u8> = Vec::new();
    let mut sizes: Vec<u32> = Vec::new();
    let mut offset_count = 0;
    while offset_count < number_offsets {
        // Offset to the the start of the data
        let (input, offset) = nom_unsigned_four_bytes(remaining_offsets, Endian::Le)?;
        remaining_offsets = input;

        let (data, _) = take(offset)(reg_data)?;
        // Get the allocated size
        let (data, (allocated, data_cell_size)) = is_allocated(data)?;

        // If data is not allocated skip it
        if !allocated {
            offset_count += 1;
            continue;
        }

        // Size includes the size itself. We nommed that away
        let adjust_cell_size = 4;
        let (_, allocated_data) = take(data_cell_size - adjust_cell_size)(data)?;
        sizes.push(data_cell_size - adjust_cell_size);
        large_data.append(&mut allocated_data.to_vec());
        offset_count += 1;
    }

    Ok((reg_data, (sizes, large_data)))
}

#[cfg(test)]
mod tests {
    use super::parse_reg_sz;
    use crate::{
        artifacts::os::windows::registry::{
            hbin::HiveBin,
            keys::data::{
                check_big_data, parse_big_data, parse_qword_filetime, parse_reg_binary,
                parse_reg_multi_sz, DataTypes,
            },
        },
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_reg_sz() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/hbins.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = HiveBin::parse_hive_bin_header(&buffer).unwrap();
        assert_eq!(result.size, 4096);

        let (_, result) = parse_reg_sz(&buffer, 78224, 24, 4).unwrap();
        assert_eq!(result, "160 160 160");
    }

    #[test]
    fn test_parse_reg_binary() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/hbins.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = HiveBin::parse_hive_bin_header(&buffer).unwrap();
        assert_eq!(result.size, 4096);

        let (_, result) = parse_reg_binary(&buffer, 83392, 712, 4).unwrap();
        assert_eq!(result, "AgAAAPQBAAABAAAAEAAAABAAAAASAAAAEgAAAPX///8AAAAAAAAAAAAAAAC8AgAAAAAAAAAAAABUAGEAaABvAG0AYQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADAAAAA8AAAD1////AAAAAAAAAAAAAAAAvAIAAAAAAAAAAAAAVABhAGgAbwBtAGEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABIAAAASAAAA9f///wAAAAAAAAAAAAAAAJABAAAAAAAAAAAAAFQAYQBoAG8AbQBhAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD1////AAAAAAAAAAAAAAAAkAEAAAAAAAAAAAAAVABhAGgAbwBtAGEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAPX///8AAAAAAAAAAAAAAACQAQAAAAAAAAAAAABUAGEAaABvAG0AYQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA9f///wAAAAAAAAAAAAAAAJABAAAAAAAAAAAAAFQAYQBoAG8AbQBhAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADU0MgAOm6lAAokagCAgIAA1NDIAP///wAAAAAAAAAAAAAAAAD///8A1NDIANTQyACAgIAACiRqAP///wDU0MgAgICAAICAgAAAAAAA1NDIAP///wBAQEAA1NDIAAAAAAD//+EAtbW1AAAAgACmyvAAwMDAAA==");
    }

    #[test]
    fn test_parse_reg_expand_sz() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/hbins.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = HiveBin::parse_hive_bin_header(&buffer).unwrap();
        assert_eq!(result.size, 4096);

        let (_, result) = parse_reg_sz(&buffer, 75824, 76, 4).unwrap();
        assert_eq!(result, "%SystemRoot%\\cursors\\aero_working.ani");
    }

    #[test]
    fn test_parse_reg_multi_sz() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/hbinsLarge.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = HiveBin::parse_hive_bin_header(&buffer).unwrap();
        assert_eq!(result.size, 4096);

        let (_, result) = parse_reg_multi_sz(&buffer, 3868224, 174, 4).unwrap();
        assert_eq!(result, "en-US\nen-IN\nen-CA\nen-GB\nen-AU\nfr-FR\nit-IT\nde-DE\nes-ES\nfr-CA\nzh-Hans-CN\nja\nes-MX\npt-BR");
    }

    #[test]
    fn test_parse_qword() {
        let test_data = [
            240, 255, 255, 255, 224, 4, 139, 204, 185, 135, 213, 1, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = parse_qword_filetime(&test_data, 0, 8, false).unwrap();
        assert_eq!(result, "132160996147660000");
    }

    #[test]
    fn test_parse_filetime() {
        let test_data = [
            240, 255, 255, 255, 224, 4, 139, 204, 185, 135, 213, 1, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = parse_qword_filetime(&test_data, 0, 8, true).unwrap();
        assert_eq!(result, "132160996147660000");
    }

    #[test]
    fn test_check_big_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/NTUSER_Large.DAT");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = HiveBin::parse_hive_bin_header(&buffer).unwrap();
        assert_eq!(result.size, 4096);

        let (_, result) = check_big_data(&buffer, 640536, 18730, DataTypes::String, 4).unwrap();
        assert_eq!(result.len(), 9364); // UTF8 half byte size of UTF16, minus end of string character
    }

    #[test]
    fn test_parse_big_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/NTUSER_Large.DAT");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = HiveBin::parse_hive_bin_header(&buffer).unwrap();
        assert_eq!(result.size, 4096);

        test_location.pop();
        test_location.push("db_data.raw");
        let db_buffer = read_file(&test_location.display().to_string()).unwrap();

        let (_, (result, result2)) = parse_big_data(&buffer, &db_buffer).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result2.len(), 32696);
    }
}
