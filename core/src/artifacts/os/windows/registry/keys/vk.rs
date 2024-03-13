use super::data::{parse_qword_filetime, parse_reg_binary, parse_reg_multi_sz, parse_reg_sz};
use crate::utils::{
    encoding::base64_encode_standard,
    nom_helper::{nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian},
    strings::{extract_ascii_utf16_string, extract_multiline_utf16_string, extract_utf16_string},
};
use nom::{
    bytes::complete::take,
    number::complete::{be_u32, le_u32},
};
use std::mem::size_of;

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
    pub(crate) fn parse_value_key<'a>(
        reg_data: &'a [u8],
        value_key: &'a [u8],
        minor_version: u32,
    ) -> nom::IResult<&'a [u8], ValueKey> {
        let (input, sig) = nom_unsigned_two_bytes(value_key, Endian::Le)?;
        let (input, name_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let (input, data_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, data_offset_data) = take(size_of::<u32>())(input)?;
        let (_, data_offset) = le_u32(data_offset_data)?;

        let (input, data_type) = nom_unsigned_four_bytes(input, Endian::Le)?;
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

        let (_, (data_type, data)) = ValueKey::get_data_and_type(
            data_type,
            data_size,
            data_offset_data,
            reg_data,
            minor_version,
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

        Ok((reg_data, value_key))
    }

    /// Get the Registry data type (Ex: `REG_SZ`) and the associated data
    fn get_data_and_type<'a>(
        data_type: u32,
        data_size: u32,
        data_offset: &'a [u8],
        reg_data: &'a [u8],
        minor_version: u32,
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
                let (_, value) = ValueKey::get_binary_data(
                    data,
                    data_size,
                    data_offset,
                    reg_data,
                    minor_version,
                )?;
                (String::from("REG_NONE"), value)
            }
            0x1 => {
                let (_, value) = ValueKey::get_string_data(
                    data,
                    data_size,
                    data_offset,
                    reg_data,
                    minor_version,
                )?;
                (String::from("REG_SZ"), value)
            }
            0x2 => {
                let (_, value) = ValueKey::get_string_data(
                    data,
                    data_size,
                    data_offset,
                    reg_data,
                    minor_version,
                )?;
                (String::from("REG_EXPAND_SZ"), value)
            }
            0x3 => {
                let (_, value) = ValueKey::get_binary_data(
                    data,
                    data_size,
                    data_offset,
                    reg_data,
                    minor_version,
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
                let (_, value) = ValueKey::get_string_data(
                    data,
                    data_size,
                    data_offset,
                    reg_data,
                    minor_version,
                )?;
                (String::from("REG_LINK"), value)
            }
            0x7 => {
                data = if data.is_empty() {
                    let (_, offset) = le_u32(data_offset)?;
                    let (_, value) =
                        parse_reg_multi_sz(reg_data, offset, data_size, minor_version)?;
                    value
                } else if data != "(NULL)" {
                    extract_multiline_utf16_string(data_offset)
                } else {
                    data
                };

                (String::from("REG_MULTI_SZ"), data)
            }
            0x8 => {
                let (_, value) = ValueKey::get_binary_data(
                    data,
                    data_size,
                    data_offset,
                    reg_data,
                    minor_version,
                )?;
                (String::from("REG_RESOURCE_LIST"), value)
            }
            0x9 => {
                let (_, value) = ValueKey::get_binary_data(
                    data,
                    data_size,
                    data_offset,
                    reg_data,
                    minor_version,
                )?;
                (String::from("REG_FULL_RESOURCE_DESCRIPTOR"), value)
            }
            0xa => {
                let (_, value) = ValueKey::get_binary_data(
                    data,
                    data_size,
                    data_offset,
                    reg_data,
                    minor_version,
                )?;
                (String::from("REG_RESOURCE_REQUIREMENTS_LIST"), value)
            }
            0xb => {
                let (_, offset) = le_u32(data_offset)?;
                let filetime = false;
                let (_, result) = parse_qword_filetime(reg_data, offset, data_size, filetime)?;
                data = result;

                (String::from("REG_QWORD"), data)
            }
            0x10 => {
                let (_, offset) = le_u32(data_offset)?;
                let filetime = true;
                let (_, result) = parse_qword_filetime(reg_data, offset, data_size, filetime)?;
                data = result;

                (String::from("REG_FILETIME"), data)
            }
            _ => {
                let (_, value) = ValueKey::get_binary_data(
                    data,
                    data_size,
                    data_offset,
                    reg_data,
                    minor_version,
                )?;
                (format!("{data_type}"), value)
            }
        };

        Ok((&[], (data_type, value_data)))
    }

    /// Get base64 encoded string that contains binary Registry data
    fn get_binary_data<'a>(
        data: String,
        data_size: u32,
        data_offset: &'a [u8],
        reg_data: &'a [u8],
        minor_version: u32,
    ) -> nom::IResult<&'a [u8], String> {
        let value = if data.is_empty() {
            let (_, offset) = le_u32(data_offset)?;
            let (_, result) = parse_reg_binary(reg_data, offset, data_size, minor_version)?;
            result
        } else if data != "(NULL)" {
            base64_encode_standard(data_offset)
        } else {
            data
        };
        Ok((reg_data, value))
    }

    /// Get string associated with Registry value data
    fn get_string_data<'a>(
        data: String,
        data_size: u32,
        data_offset: &'a [u8],
        reg_data: &'a [u8],
        minor_version: u32,
    ) -> nom::IResult<&'a [u8], String> {
        let value = if data.is_empty() {
            let (_, offset) = le_u32(data_offset)?;
            let (_, value) = parse_reg_sz(reg_data, offset, data_size, minor_version)?;
            value
        } else if data != "(NULL)" {
            extract_utf16_string(data_offset)
        } else {
            data
        };
        Ok((reg_data, value))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::registry::{hbin::HiveBin, keys::vk::ValueKey},
        filesystem::files::read_file,
    };
    use std::path::PathBuf;

    #[test]
    fn test_parse_value_key() {
        let test_data = [
            118, 107, 12, 0, 24, 0, 0, 0, 144, 49, 1, 0, 1, 0, 0, 0, 1, 0, 0, 0, 66, 117, 116, 116,
            111, 110, 83, 104, 97, 100, 111, 119, 0, 0, 0, 0,
        ];
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/hbins.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = HiveBin::parse_hive_bin_header(&buffer).unwrap();

        assert_eq!(result.size, 4096);
        let (_, result) = ValueKey::parse_value_key(&buffer, &test_data, 4).unwrap();
        assert_eq!(result.data, "160 160 160");
        assert_eq!(result.value_name, "ButtonShadow");
        assert_eq!(result._sig, 0x6b76); //vk
        assert_eq!(result._name_size, 12);
        assert_eq!(result._data_size, 24);
        assert_eq!(result._data_offset, 78224);
        assert_eq!(result.data_type, "REG_SZ");
        assert_eq!(result._flags, 1);
        assert_eq!(result._padding, 0);
    }

    #[test]
    fn test_get_data_and_type() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/hbins.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = HiveBin::parse_hive_bin_header(&buffer).unwrap();

        assert_eq!(result.size, 4096);
        let (_, (data_type, data)) =
            ValueKey::get_data_and_type(1, 24, &[144, 49, 1, 0], &buffer, 4).unwrap();
        assert_eq!(data_type, "REG_SZ");
        assert_eq!(data, "160 160 160");
    }

    #[test]
    fn test_get_string_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/hbins.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = HiveBin::parse_hive_bin_header(&buffer).unwrap();

        assert_eq!(result.size, 4096);
        let (_, data) =
            ValueKey::get_string_data(String::new(), 24, &[144, 49, 1, 0], &buffer, 4).unwrap();
        assert_eq!(data, "160 160 160");
    }

    #[test]
    fn test_get_binary_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/registry/win10/hbins.raw");

        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, result) = HiveBin::parse_hive_bin_header(&buffer).unwrap();

        assert_eq!(result.size, 4096);
        let (_, data) =
            ValueKey::get_binary_data(String::new(), 712, &[192, 69, 1, 0], &buffer, 4).unwrap();

        assert_eq!(data, "AgAAAPQBAAABAAAAEAAAABAAAAASAAAAEgAAAPX///8AAAAAAAAAAAAAAAC8AgAAAAAAAAAAAABUAGEAaABvAG0AYQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADAAAAA8AAAD1////AAAAAAAAAAAAAAAAvAIAAAAAAAAAAAAAVABhAGgAbwBtAGEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAABIAAAASAAAA9f///wAAAAAAAAAAAAAAAJABAAAAAAAAAAAAAFQAYQBoAG8AbQBhAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAD1////AAAAAAAAAAAAAAAAkAEAAAAAAAAAAAAAVABhAGgAbwBtAGEAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAPX///8AAAAAAAAAAAAAAACQAQAAAAAAAAAAAABUAGEAaABvAG0AYQAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA9f///wAAAAAAAAAAAAAAAJABAAAAAAAAAAAAAFQAYQBoAG8AbQBhAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAADU0MgAOm6lAAokagCAgIAA1NDIAP///wAAAAAAAAAAAAAAAAD///8A1NDIANTQyACAgIAACiRqAP///wDU0MgAgICAAICAgAAAAAAA1NDIAP///wBAQEAA1NDIAAAAAAD//+EAtbW1AAAAgACmyvAAwMDAAA==");
    }
}
