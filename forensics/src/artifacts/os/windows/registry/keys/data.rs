use crate::{
    artifacts::os::windows::registry::{cell::is_allocated, error::RegistryError},
    filesystem::ntfs::reader::read_bytes,
    utils::{
        encoding::base64_encode_standard,
        nom_helper::{
            Endian, nom_data, nom_signed_eight_bytes, nom_unsigned_eight_bytes,
            nom_unsigned_four_bytes, nom_unsigned_two_bytes,
        },
        strings::{extract_multiline_utf16_string, extract_utf16_string},
        time::{filetime_to_unixepoch, unixepoch_to_iso},
    },
};
use log::error;
use ntfs::NtfsFile;
use std::io::BufReader;

#[derive(PartialEq)]
enum DataTypes {
    MultiString,
    String,
    Binary,
}

/// Parse Registry data that contains a qword (64 bit) data.
pub(crate) fn parse_qword_filetime_reader<'a, T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'_>>,
    offset: u32,
    data_size: u32,
    filetime: bool,
    size: u32,
) -> Result<String, RegistryError> {
    let input = match read_bytes(offset as u64, size as u64, ntfs_file, reader) {
        Ok(result) => result,
        Err(err) => {
            error!("[registry] Failed to read qword bytes: {err:?}");
            return Err(RegistryError::ReadRegistry);
        }
    };

    let (input, (allocated, data_cell_size)) = match is_allocated(&input) {
        Ok(result) => result,
        Err(_err) => {
            error!("[registry] Failed to check if qword allocated");
            return Err(RegistryError::Parser);
        }
    };

    if !allocated {
        error!("[registry] Got unallocated FILETIME/QWORD data");
        return Ok(String::new());
    }
    // Size includes the size itself. We nommed that away
    let adjust_cell_size = 4;
    if data_cell_size < adjust_cell_size {
        return Err(RegistryError::Parser);
    }
    let (_, allocated_data) = match nom_data(input, (data_cell_size - adjust_cell_size) as u64) {
        Ok(result) => result,
        Err(_err) => {
            error!("[registry] Failed to get qword data");
            return Err(RegistryError::Parser);
        }
    };

    // The size in the Value key is the actual size of the data
    // Any remaining data is slack space
    let (_slack_space, allocated_data) = match nom_data(allocated_data, data_size as u64) {
        Ok(result) => result,
        Err(_err) => {
            error!("[registry] Failed to get allocated qword data");
            return Err(RegistryError::Parser);
        }
    };

    let value = if filetime {
        let (_, value) = match nom_unsigned_eight_bytes(allocated_data, Endian::Le) {
            Ok(result) => result,
            Err(_err) => {
                error!("[registry] Failed to get allocated qword filetime");
                return Err(RegistryError::Parser);
            }
        };
        let reg_time = filetime_to_unixepoch(value);
        unixepoch_to_iso(reg_time)
    } else {
        let (_, value) = match nom_signed_eight_bytes(allocated_data, Endian::Le) {
            Ok(result) => result,
            Err(_err) => {
                error!("[registry] Failed to parse allocated qword");
                return Err(RegistryError::Parser);
            }
        };
        format!("{value}")
    };
    Ok(value)
}

/// Parse Registry data that contains a string
pub(crate) fn parse_reg_sz_reader<'a, T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'_>>,
    offset: u32,
    data_size: u32,
    minor_version: u32,
    size: u32,
) -> Result<String, RegistryError> {
    let data_type = DataTypes::String;
    let value = check_big_data_reader(
        reader,
        ntfs_file,
        offset,
        data_size,
        data_type,
        minor_version,
        size,
    )?;

    Ok(value)
}

/// Parse Registry data that contains binary data. Return as base64 encoded string
pub(crate) fn parse_reg_binary_reader<'a, T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'_>>,
    offset: u32,
    data_size: u32,
    minor_version: u32,
    size: u32,
) -> Result<String, RegistryError> {
    let data_type = DataTypes::Binary;
    let value = check_big_data_reader(
        reader,
        ntfs_file,
        offset,
        data_size,
        data_type,
        minor_version,
        size,
    )?;

    Ok(value)
}

/// Parse Registry data that contains a multi-line string
pub(crate) fn parse_reg_multi_sz_reader<'a, T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'_>>,
    offset: u32,
    data_size: u32,
    minor_version: u32,
    size: u32,
) -> Result<String, RegistryError> {
    let data_type = DataTypes::MultiString;
    let value = check_big_data_reader(
        reader,
        ntfs_file,
        offset,
        data_size,
        data_type,
        minor_version,
        size,
    )?;

    Ok(value)
}

/// Setup the big data reader
fn check_big_data_reader<'a, T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'_>>,
    offset: u32,
    data_size: u32,
    data_type: DataTypes,
    minor_version: u32,
    size: u32,
) -> Result<String, RegistryError> {
    let offset_list = match read_bytes(offset as u64, size as u64, ntfs_file, reader) {
        Ok(result) => result,
        Err(err) => {
            error!("[registry] Failed to read and check big data bytes: {err:?}");
            return Err(RegistryError::ReadRegistry);
        }
    };

    let (input, (allocated, data_cell_size)) = match is_allocated(&offset_list) {
        Ok(result) => result,
        Err(_err) => {
            error!("[registry] Failed to check if big data allocated");
            return Err(RegistryError::Parser);
        }
    };

    if !allocated {
        error!("[registry] Got unallocated Big Data");
        return Ok(String::new());
    }
    // The max size of big data is 16344
    let max_size = 16344;
    let min_version = 3;

    // If Value data is very large, then it is stored in a Db cell list
    // Also Db cell list only exists in Registry versions higher than 1.3.
    if data_size > max_size && minor_version > min_version {
        // large_data contains the whole data for the value key. Even possible padding and slack data (for large strings)
        let (sizes, large_data) = big_data_reader(reader, ntfs_file, input, size)?;

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
                let end_string = 2;
                // The final size should be the difference between the data size specified in the value key and our current data
                let final_size = if data_type == DataTypes::Binary {
                    data_size as usize - binary_vec.len()
                } else {
                    (data_size as usize - start_size) + end_string
                };

                // Adjust the size based on final data size
                let allocated_data = if final_size + start_size < large_data.len()
                    && final_size + start_size > start_size
                {
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

        return Ok(reg_string);
    }

    // Size includes the size itself. We nommed that away
    let adjust_cell_size = 4;
    if data_cell_size < adjust_cell_size {
        return Err(RegistryError::Parser);
    }
    let allocated_data = match read_bytes(
        (offset + adjust_cell_size) as u64,
        data_cell_size as u64,
        ntfs_file,
        reader,
    ) {
        Ok(result) => result,
        Err(err) => {
            error!("[registry] Failed to read and check big data bytes: {err:?}");
            return Err(RegistryError::ReadRegistry);
        }
    };

    // The size in the Value key is the actual size of the data
    // Any remaining data is slack space
    let (_slack_space, allocated_data) = match nom_data(&allocated_data, data_size as u64) {
        Ok(result) => result,
        Err(_err) => {
            error!("[registry] Failed to get allocated big data value");
            return Err(RegistryError::Parser);
        }
    };
    let value = if data_type == DataTypes::String {
        extract_utf16_string(allocated_data)
    } else if data_type == DataTypes::MultiString {
        extract_multiline_utf16_string(allocated_data)
    } else {
        base64_encode_standard(allocated_data)
    };
    Ok(value)
}

/// Parse and get big data
fn big_data_reader<'a, T: std::io::Seek + std::io::Read>(
    reader: &mut BufReader<T>,
    ntfs_file: Option<&NtfsFile<'_>>,
    db_data: &'a [u8],
    size: u32,
) -> Result<(Vec<u32>, Vec<u8>), RegistryError> {
    let (input, _sig) = match nom_unsigned_two_bytes(db_data, Endian::Le) {
        Ok(result) => result,
        Err(_err) => {
            error!("[registry] Failed to get big data sig");
            return Err(RegistryError::Parser);
        }
    };
    let (input, number_offsets) = match nom_unsigned_two_bytes(input, Endian::Le) {
        Ok(result) => result,
        Err(_err) => {
            error!("[registry] Failed to get big data offset count");
            return Err(RegistryError::Parser);
        }
    };

    // Offset to a list of offsets that all point to the large registry value data
    let (_, offset) = match nom_unsigned_four_bytes(input, Endian::Le) {
        Ok(result) => result,
        Err(_err) => {
            error!("[registry] Failed to get big data offset");
            return Err(RegistryError::Parser);
        }
    };

    let offset_list = match read_bytes((offset + size) as u64, size as u64, ntfs_file, reader) {
        Ok(result) => result,
        Err(err) => {
            error!("[registry] Failed to read big data bytes: {err:?}");
            return Err(RegistryError::ReadRegistry);
        }
    };

    let (mut remaining_offsets, (allocated, _)) = match is_allocated(&offset_list) {
        Ok(result) => result,
        Err(_err) => {
            error!("[registry] Failed to determine if big data allocated");
            return Err(RegistryError::Parser);
        }
    };

    // If offset_list is not allocated, just return
    if !allocated {
        return Ok((Vec::new(), Vec::new()));
    }

    // Remaining values in the offset_list are offsets to the registry data
    // Store and track all data associated with large value key
    let mut large_data: Vec<u8> = Vec::new();
    let mut sizes: Vec<u32> = Vec::new();
    let mut offset_count = 0;
    while offset_count < number_offsets {
        // Offset to the the start of the data
        let (input, offset) = match nom_unsigned_four_bytes(remaining_offsets, Endian::Le) {
            Ok(result) => result,
            Err(_err) => {
                error!("[registry] Failed to get big data next offset");
                return Err(RegistryError::Parser);
            }
        };
        remaining_offsets = input;

        let data = match read_bytes((offset + size) as u64, size as u64, ntfs_file, reader) {
            Ok(result) => result,
            Err(err) => {
                panic!(
                    "[registry] Failed to read full big data. Data will be incomplete!: {err:?}"
                );
                offset_count += 1;
                continue;
            }
        };

        // Get the allocated size
        let (data, (allocated, data_cell_size)) = match is_allocated(&data) {
            Ok(result) => result,
            Err(_err) => {
                error!("[registry] Failed to get next big data offset");
                return Err(RegistryError::Parser);
            }
        };

        // If data is not allocated skip it
        if !allocated {
            offset_count += 1;
            continue;
        }

        // Size includes the size itself. We nommed that away
        let adjust_cell_size = 4;
        if data_cell_size < adjust_cell_size {
            return Err(RegistryError::Parser);
        }

        let allocated_data = match read_bytes(
            (offset + size + adjust_cell_size) as u64,
            (data_cell_size - adjust_cell_size) as u64,
            ntfs_file,
            reader,
        ) {
            Ok(result) => result,
            Err(err) => {
                panic!(
                    "[registry] Failed to read full big data. Data will be incomplete!: {err:?}"
                );
                offset_count += 1;
                continue;
            }
        };

        sizes.push(data_cell_size - adjust_cell_size);
        large_data.append(&mut allocated_data.to_vec());
        offset_count += 1;
    }

    Ok((sizes, large_data))
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::registry::{
            keys::data::{
                DataTypes, big_data_reader, check_big_data_reader, parse_qword_filetime_reader,
                parse_reg_binary_reader, parse_reg_multi_sz_reader, parse_reg_sz_reader,
            },
            reader::setup_registry_reader,
        },
        filesystem::files::read_file,
    };
    use std::{
        io::{BufReader, Cursor},
        path::PathBuf,
    };

    #[test]
    fn test_parse_reg_sz_reader() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/hbins.raw");

        let reader = setup_registry_reader(test_location.to_str().unwrap()).unwrap();
        let mut buf_reader = BufReader::new(reader);

        let result = parse_reg_sz_reader(&mut buf_reader, None, 78224, 24, 3, 4096).unwrap();
        assert_eq!(result, "160 160 160");
    }

    #[test]
    fn test_parse_reg_binary_reader() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/hbins.raw");

        let reader = setup_registry_reader(test_location.to_str().unwrap()).unwrap();
        let mut buf_reader = BufReader::new(reader);

        let result = parse_reg_binary_reader(&mut buf_reader, None, 83392, 712, 4, 4096).unwrap();
        assert_eq!(
            result,
            "AgAAAPQBAAABAAAAEAAAABAAAAASAAAAEgAAAPX///8AAAAAAAAAAAAAAAC8AgAAAAAAAAAAAABUAGEAaABvAG0AYQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADAAAAA8AAAD1////AAAAAAAAAAAAAAAAvAIAAAAAAAAAAAAAVABhAGgAbwBtAGEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABIAAAASAAAA9f///wAAAAAAAAAAAAAAAJABAAAAAAAAAAAAAFQAYQBoAG8AbQBhAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD1////AAAAAAAAAAAAAAAAkAEAAAAAAAAAAAAAVABhAGgAbwBtAGEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAPX///8AAAAAAAAAAAAAAACQAQAAAAAAAAAAAABUAGEAaABvAG0AYQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA9f///wAAAAAAAAAAAAAAAJABAAAAAAAAAAAAAFQAYQBoAG8AbQBhAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADU0MgAOm6lAAokagCAgIAA1NDIAP///wAAAAAAAAAAAAAAAAD///8A1NDIANTQyACAgIAACiRqAP///wDU0MgAgICAAICAgAAAAAAA1NDIAP///wBAQEAA1NDIAAAAAAD//+EAtbW1AAAAgACmyvAAwMDAAA=="
        );
    }

    #[test]
    fn test_parse_reg_expand_sz_reader() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/hbins.raw");

        let reader = setup_registry_reader(test_location.to_str().unwrap()).unwrap();
        let mut buf_reader = BufReader::new(reader);

        let result = parse_reg_sz_reader(&mut buf_reader, None, 75824, 76, 4, 4096).unwrap();
        assert_eq!(result, "%SystemRoot%\\cursors\\aero_working.ani");
    }

    #[test]
    fn test_parse_reg_multi_sz_reader() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/hbinsLarge.raw");

        let reader = setup_registry_reader(test_location.to_str().unwrap()).unwrap();
        let mut buf_reader = BufReader::new(reader);

        let result =
            parse_reg_multi_sz_reader(&mut buf_reader, None, 3868224, 174, 4, 4096).unwrap();
        assert_eq!(
            result,
            "en-US\nen-IN\nen-CA\nen-GB\nen-AU\nfr-FR\nit-IT\nde-DE\nes-ES\nfr-CA\nzh-Hans-CN\nja\nes-MX\npt-BR"
        );
    }

    #[test]
    fn test_parse_qword_reader() {
        let test_data = [
            240, 255, 255, 255, 224, 4, 139, 204, 185, 135, 213, 1, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let mut buf_reader = BufReader::new(Cursor::new(&test_data));

        let result = parse_qword_filetime_reader(&mut buf_reader, None, 0, 8, false, 20).unwrap();
        assert_eq!(result, "132160996147660000");
    }

    #[test]
    fn test_parse_filetime_reader() {
        let test_data = [
            240, 255, 255, 255, 224, 4, 139, 204, 185, 135, 213, 1, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let mut buf_reader = BufReader::new(Cursor::new(&test_data));

        let result = parse_qword_filetime_reader(&mut buf_reader, None, 0, 8, true, 20).unwrap();
        assert_eq!(result, "2019-10-21T02:46:54.000Z");
    }

    #[test]
    fn test_check_big_data_reader() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/NTUSER_Large.DAT");

        let reader = setup_registry_reader(test_location.to_str().unwrap()).unwrap();
        let mut buf_reader = BufReader::new(reader);

        let result = check_big_data_reader(
            &mut buf_reader,
            None,
            644632,
            18730,
            DataTypes::String,
            4,
            4096,
        )
        .unwrap();
        assert_eq!(result.len(), 9364); // UTF8 half byte size of UTF16, minus end of string character
    }

    #[test]
    fn test_big_data_reader() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/NTUSER_Large.DAT");

        let reader = setup_registry_reader(test_location.to_str().unwrap()).unwrap();
        let mut buf_reader = BufReader::new(reader);

        test_location.pop();
        test_location.push("db_data.raw");
        let db_buffer = read_file(&test_location.display().to_string()).unwrap();

        let (result, result2) = big_data_reader(&mut buf_reader, None, &db_buffer, 4096).unwrap();
        assert_eq!(result.len(), 2);
        assert_eq!(result2.len(), 32696);
    }
}
