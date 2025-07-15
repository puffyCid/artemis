use crate::{
    artifacts::os::windows::registry::keys::data::{
        parse_qword_filetime_reader, parse_reg_binary_reader, parse_reg_multi_sz_reader,
        parse_reg_sz_reader,
    },
    utils::{
        encoding::base64_encode_standard,
        nom_helper::{Endian, nom_unsigned_four_bytes, nom_unsigned_two_bytes},
        strings::{
            extract_ascii_utf16_string, extract_multiline_utf16_string, extract_utf16_string,
        },
    },
};
use log::error;
use nom::{
    bytes::complete::take,
    error::ErrorKind,
    number::complete::{be_u32, le_u32},
};
use ntfs::NtfsFile;
use std::{io::BufReader, mem::size_of};

#[derive(Debug)]
pub(crate) struct ValueKey {
    _sig: u16,
    pub(crate) _name_size: u16, // If value is zero (0), name is "(default)"
    pub(crate) _data_size: u32, // Size of zero (0) means its NULL
    pub(crate) _data_offset: u32,
    pub(crate) data_type: String,
    pub(crate) _flags: u16,
    _padding: u16,
    pub(crate) value_name: String, // ASCII or UTF16
    pub(crate) data: String,
}

impl ValueKey {
    /// Parse the Value key data and get any data associated with the key
    pub(crate) fn value_key_reader<'a, T: std::io::Seek + std::io::Read>(
        reader: &mut BufReader<T>,
        ntfs_file: Option<&NtfsFile<'_>>,
        value_data: &'a [u8],
        minor_version: u32,
        size: u32,
    ) -> nom::IResult<&'a [u8], ValueKey> {
        let (input, sig) = nom_unsigned_two_bytes(value_data, Endian::Le)?;
        let (input, name_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, data_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, data_offset_data) = take(size_of::<u32>())(input)?;
        let (_, data_offset) = le_u32(data_offset_data)?;

        let (input, mut data_type) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, flags) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, padding) = nom_unsigned_two_bytes(input, Endian::Le)?;

        let default_name = 0;
        let value_name = if name_size == default_name {
            String::from("(default)")
        } else {
            let (_, value_name_bytes) = take(name_size)(input)?;
            // Value name can be ASCII or UTF16
            extract_ascii_utf16_string(value_name_bytes)
        };

        let dev_prop = 0xffff0000;
        // Check if a devprop structure?
        // https://github.com/mkorman90/regipy/blob/master/regipy/registry.py#L462
        if data_type > dev_prop {
            data_type &= 0xffff;
        }

        let (_, (data_type, data)) = ValueKey::data_reader(
            reader,
            ntfs_file,
            data_type,
            data_size,
            data_offset_data,
            minor_version,
            size,
        )?;

        let value_key = ValueKey {
            _sig: sig,
            _name_size: name_size,
            _data_size: data_size,
            _data_offset: data_offset,
            data_type,
            _flags: flags,
            _padding: padding,
            value_name,
            data,
        };

        Ok((input, value_key))
    }

    fn data_reader<'a, T: std::io::Seek + std::io::Read>(
        reader: &mut BufReader<T>,
        ntfs_file: Option<&NtfsFile<'_>>,
        data_type: u32,
        data_size: u32,
        data_offset: &'a [u8],
        minor_version: u32,
        size: u32,
    ) -> nom::IResult<&'a [u8], (String, String)> {
        let null_size = 0;
        let mut data = if data_size == null_size {
            String::from("(NULL)") // Data value is "(NULL)" if data size is zero (0)
        } else {
            let resident_check = 0x80000000;
            if (data_size & resident_check) == resident_check {
                String::from("resident") // If the data is small enough to fit in the offset size, then the offset (data_offset) contains our data
            } else {
                String::new() // Otherwise we need to go to the data_offset to grab the data
            }
        };

        let (data_type, value_data) = match data_type {
            0x0 => {
                let (_, value) = ValueKey::get_binary_data_reader(
                    reader,
                    ntfs_file,
                    data,
                    data_size,
                    data_offset,
                    minor_version,
                    size,
                )?;
                (String::from("REG_NONE"), value)
            }
            0x1 => {
                let (_, value) = ValueKey::get_string_data_reader(
                    reader,
                    ntfs_file,
                    data,
                    data_size,
                    data_offset,
                    minor_version,
                    size,
                )?;
                (String::from("REG_SZ"), value)
            }
            0x2 => {
                let (_, value) = ValueKey::get_string_data_reader(
                    reader,
                    ntfs_file,
                    data,
                    data_size,
                    data_offset,
                    minor_version,
                    size,
                )?;
                (String::from("REG_EXPAND_SZ"), value)
            }
            0x3 => {
                let (_, value) = ValueKey::get_binary_data_reader(
                    reader,
                    ntfs_file,
                    data,
                    data_size,
                    data_offset,
                    minor_version,
                    size,
                )?;
                (String::from("REG_BINARY"), value)
            }
            0x4 => {
                // REG_DWORD values should be stored in the data offset since there size is 4 bytes or fewer
                let (_, value) = le_u32(data_offset)?;
                data = format!("{value}");
                (String::from("REG_DWORD"), data)
            }
            0x5 => {
                // REG_DWORD_BIG_ENDIAN values should be stored in the data offset since there size is 4 bytes or fewer
                let (_, value) = be_u32(data_offset)?;
                data = format!("{value}");
                (String::from("REG_DWORD_BIG_ENDIAN"), data)
            }
            0x6 => {
                let (_, value) = ValueKey::get_string_data_reader(
                    reader,
                    ntfs_file,
                    data,
                    data_size,
                    data_offset,
                    minor_version,
                    size,
                )?;
                (String::from("REG_LINK"), value)
            }
            0x7 => {
                data = if data.is_empty() {
                    let (_, offset) = le_u32(data_offset)?;
                    let value = match parse_reg_multi_sz_reader(
                        reader,
                        ntfs_file,
                        offset + size,
                        data_size,
                        minor_version,
                        size,
                    ) {
                        Ok(result) => result,
                        Err(err) => {
                            error!("[registry] Failed to get multi size string: {err:?}");
                            return Err(nom::Err::Failure(nom::error::Error::new(
                                &[],
                                ErrorKind::Fail,
                            )));
                        }
                    };
                    value
                } else if data != "(NULL)" {
                    extract_multiline_utf16_string(data_offset)
                } else {
                    data
                };

                (String::from("REG_MULTI_SZ"), data)
            }
            0x8 => {
                let (_, value) = ValueKey::get_binary_data_reader(
                    reader,
                    ntfs_file,
                    data,
                    data_size,
                    data_offset,
                    minor_version,
                    size,
                )?;
                (String::from("REG_RESOURCE_LIST"), value)
            }
            0x9 => {
                let (_, value) = ValueKey::get_binary_data_reader(
                    reader,
                    ntfs_file,
                    data,
                    data_size,
                    data_offset,
                    minor_version,
                    size,
                )?;
                (String::from("REG_FULL_RESOURCE_DESCRIPTOR"), value)
            }
            0xa => {
                let (_, value) = ValueKey::get_binary_data_reader(
                    reader,
                    ntfs_file,
                    data,
                    data_size,
                    data_offset,
                    minor_version,
                    size,
                )?;
                (String::from("REG_RESOURCE_REQUIREMENTS_LIST"), value)
            }
            0xb => {
                let (_, offset) = le_u32(data_offset)?;
                let filetime = false;
                //let (_, result) = parse_qword_filetime(reg_data, offset, data_size, filetime)?;
                let result = match parse_qword_filetime_reader(
                    reader,
                    ntfs_file,
                    offset + size,
                    data_size,
                    filetime,
                    size,
                ) {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[registry] Failed to get qword registry value: {err:?}");
                        return Err(nom::Err::Failure(nom::error::Error::new(
                            &[],
                            ErrorKind::Fail,
                        )));
                    }
                };
                data = result;

                (String::from("REG_QWORD"), data)
            }
            0x10 => {
                let (_, offset) = le_u32(data_offset)?;
                let filetime = true;
                //let (_, result) = parse_qword_filetime(reg_data, offset, data_size, filetime)?;
                let result = match parse_qword_filetime_reader(
                    reader,
                    ntfs_file,
                    offset + size,
                    data_size,
                    filetime,
                    size,
                ) {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[registry] Failed to get filetime registry value: {err:?}");
                        return Err(nom::Err::Failure(nom::error::Error::new(
                            &[],
                            ErrorKind::Fail,
                        )));
                    }
                };
                data = result;

                (String::from("REG_FILETIME"), data)
            }
            _ => {
                let (_, value) = ValueKey::get_binary_data_reader(
                    reader,
                    ntfs_file,
                    data,
                    data_size,
                    data_offset,
                    minor_version,
                    size,
                )?;
                (format!("{data_type}"), value)
            }
        };

        Ok((&[], (data_type, value_data)))
    }

    /// Get base64 encoded string that contains binary Registry data
    fn get_binary_data_reader<'a, T: std::io::Seek + std::io::Read>(
        reader: &mut BufReader<T>,
        ntfs_file: Option<&NtfsFile<'_>>,
        data: String,
        data_size: u32,
        data_offset: &'a [u8],
        minor_version: u32,
        size: u32,
    ) -> nom::IResult<&'a [u8], String> {
        let value = if data.is_empty() {
            let (_, offset) = le_u32(data_offset)?;
            let result = match parse_reg_binary_reader(
                reader,
                ntfs_file,
                offset + size,
                data_size,
                minor_version,
                size,
            ) {
                Ok(result) => result,
                Err(err) => {
                    error!("[registry] Failed to get binary registry value: {err:?}");
                    return Err(nom::Err::Failure(nom::error::Error::new(
                        &[],
                        ErrorKind::Fail,
                    )));
                }
            };
            result
        } else if data != "(NULL)" {
            base64_encode_standard(data_offset)
        } else {
            data
        };
        Ok((&[], value))
    }

    /// Get string associated with Registry value data
    fn get_string_data_reader<'a, T: std::io::Seek + std::io::Read>(
        reader: &mut BufReader<T>,
        ntfs_file: Option<&NtfsFile<'_>>,
        data: String,
        data_size: u32,
        data_offset: &'a [u8],
        minor_version: u32,
        size: u32,
    ) -> nom::IResult<&'a [u8], String> {
        let value = if data.is_empty() {
            let (_, offset) = le_u32(data_offset)?;
            //let (_, value) = parse_reg_sz(reg_data, offset, data_size, minor_version)?;
            let value = match parse_reg_sz_reader(
                reader,
                ntfs_file,
                offset + size,
                data_size,
                minor_version,
                size,
            ) {
                Ok(result) => result,
                Err(err) => {
                    error!("[registry] Failed to get string registry value: {err:?}");
                    return Err(nom::Err::Failure(nom::error::Error::new(
                        &[],
                        ErrorKind::Fail,
                    )));
                }
            };
            value
        } else if data != "(NULL)" {
            extract_utf16_string(data_offset)
        } else {
            data
        };
        Ok((&[], value))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::registry::{hbin::HiveBin, keys::vk::ValueKey},
        filesystem::files::read_file,
    };
    use std::path::PathBuf;
}
