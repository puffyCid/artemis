use crate::artifacts::os::windows::propertystore::parser::get_property_guid;
use crate::utils::nom_helper::Endian;
use crate::utils::nom_helper::nom_unsigned_two_bytes;
use crate::utils::strings::extract_utf8_string;
use common::windows::ShellItem;
use common::windows::ShellType;
use log::error;
use nom::bytes::complete::take;
use std::mem::size_of;

/// Parse a `Property` `ShellItem`. These are very complex structures. Currently only getting the first GUID
pub(crate) fn parse_property(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let (input, _unknown) = take(size_of::<u8>())(data)?;
    let (input, _data_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, _signature) = take(size_of::<u32>())(input)?;
    let (input, _property_size) = take(size_of::<u16>())(input)?;

    let (input, id_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
    let (input, _id_data) = take(id_size)(input)?;

    let result = get_property_guid(input);
    let stores = match result {
        Ok(results) => results,
        Err(err) => {
            error!("[shellitems] Could not get property store shellitem:{err:?}");
            Vec::new()
        }
    };

    let store_item = ShellItem {
        value: String::from("Property View"),
        shell_type: ShellType::UserPropertyView,
        created: String::from("1970-01-01T00:00:00.000Z"),
        modified: String::from("1970-01-01T00:00:00.000Z"),
        accessed: String::from("1970-01-01T00:00:00.000Z"),
        mft_entry: 0,
        mft_sequence: 0,
        stores,
    };

    Ok((input, store_item))
}

/// Parse a `Property` `ShellItem` that contains a drive letter
pub(crate) fn parse_property_drive(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let drive_offset: u8 = 10;
    let drive_size: u8 = 3;

    let (input, _) = take(drive_offset)(data)?;
    let (input, drive_data) = take(drive_size)(input)?;

    let drive = extract_utf8_string(drive_data);
    let store_item = ShellItem {
        value: drive,
        shell_type: ShellType::UserPropertyView,
        created: String::from("1970-01-01T00:00:00.000Z"),
        modified: String::from("1970-01-01T00:00:00.000Z"),
        accessed: String::from("1970-01-01T00:00:00.000Z"),
        mft_entry: 0,
        mft_sequence: 0,
        stores: Vec::new(),
    };
    Ok((input, store_item))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::shellitems::property::{
        parse_property, parse_property_drive,
    };
    use common::windows::ShellType;

    #[test]
    fn test_parse_property() {
        let test_data = [
            0, 249, 2, 213, 223, 163, 35, 235, 2, 4, 0, 0, 0, 0, 0, 231, 2, 0, 0, 49, 83, 80, 83,
            5, 213, 205, 213, 156, 46, 27, 16, 147, 151, 8, 0, 43, 44, 249, 174, 39, 2, 0, 0, 18,
            0, 0, 0, 0, 65, 0, 117, 0, 116, 0, 111, 0, 76, 0, 105, 0, 115, 0, 116, 0, 0, 0, 66, 0,
            0, 0, 30, 0, 0, 0, 112, 0, 114, 0, 111, 0, 112, 0, 52, 0, 50, 0, 57, 0, 52, 0, 57, 0,
            54, 0, 55, 0, 50, 0, 57, 0, 53, 0, 0, 0, 0, 0, 221, 1, 0, 0, 174, 165, 78, 56, 225,
            173, 138, 78, 138, 155, 123, 234, 120, 255, 241, 233, 6, 0, 0, 128, 0, 0, 0, 0, 1, 0,
            0, 0, 2, 0, 0, 128, 1, 0, 0, 0, 1, 0, 0, 0, 2, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 223,
            0, 20, 0, 31, 80, 224, 79, 208, 32, 234, 58, 105, 16, 162, 216, 8, 0, 43, 48, 48, 157,
            25, 0, 47, 67, 58, 92, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 86, 0,
            49, 0, 0, 0, 0, 0, 183, 80, 11, 162, 16, 0, 87, 105, 110, 100, 111, 119, 115, 0, 64, 0,
            9, 0, 4, 0, 239, 190, 115, 78, 172, 36, 183, 80, 11, 162, 46, 0, 0, 0, 87, 146, 1, 0,
            0, 0, 6, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 244, 161, 252, 0, 87, 0, 105, 0,
            110, 0, 100, 0, 111, 0, 119, 0, 115, 0, 0, 0, 22, 0, 90, 0, 49, 0, 0, 0, 0, 0, 203, 80,
            102, 10, 16, 0, 83, 121, 115, 116, 101, 109, 51, 50, 0, 0, 66, 0, 9, 0, 4, 0, 239, 190,
            115, 78, 172, 36, 203, 80, 102, 10, 46, 0, 0, 0, 147, 155, 1, 0, 0, 0, 10, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 133, 88, 172, 0, 83, 0, 121, 0, 115, 0, 116, 0, 101,
            0, 109, 0, 51, 0, 50, 0, 0, 0, 24, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 1, 0, 0, 128, 1, 0, 0, 0, 4, 0, 105, 0, 116, 0, 101, 0,
            109, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 30, 26, 222, 127, 49, 139, 165, 73, 147, 184,
            107, 225, 76, 250, 73, 67, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255, 255,
            0, 0, 0, 0, 1, 0, 0, 0, 26, 0, 83, 0, 101, 0, 97, 0, 114, 0, 99, 0, 104, 0, 32, 0, 82,
            0, 101, 0, 115, 0, 117, 0, 108, 0, 116, 0, 115, 0, 32, 0, 105, 0, 110, 0, 32, 0, 83, 0,
            121, 0, 115, 0, 116, 0, 101, 0, 109, 0, 51, 0, 50, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 57, 0, 0, 0, 36, 0, 0, 0,
            0, 65, 0, 117, 0, 116, 0, 111, 0, 108, 0, 105, 0, 115, 0, 116, 0, 67, 0, 97, 0, 99, 0,
            104, 0, 101, 0, 84, 0, 105, 0, 109, 0, 101, 0, 0, 0, 20, 0, 0, 0, 149, 78, 49, 203, 24,
            0, 0, 0, 107, 0, 0, 0, 34, 0, 0, 0, 0, 65, 0, 117, 0, 116, 0, 111, 0, 108, 0, 105, 0,
            115, 0, 116, 0, 67, 0, 97, 0, 99, 0, 104, 0, 101, 0, 75, 0, 101, 0, 121, 0, 0, 0, 31,
            0, 0, 0, 28, 0, 0, 0, 83, 0, 101, 0, 97, 0, 114, 0, 99, 0, 104, 0, 32, 0, 82, 0, 101,
            0, 115, 0, 117, 0, 108, 0, 116, 0, 115, 0, 32, 0, 105, 0, 110, 0, 32, 0, 83, 0, 121, 0,
            115, 0, 116, 0, 101, 0, 109, 0, 51, 0, 50, 0, 48, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 116, 26, 89, 94, 150, 223, 211, 72, 141, 103, 23, 51, 188, 238, 40, 186, 103, 27,
            115, 4, 51, 217, 10, 69, 144, 230, 74, 205, 46, 148, 8, 254, 42, 0, 0, 0, 19, 0, 239,
            190, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 1, 0, 0, 0, 31, 3, 0, 0,
        ];

        let (_, result) = parse_property(&test_data).unwrap();
        assert_eq!(result.value, "Property View");
        assert_eq!(result.shell_type, ShellType::UserPropertyView);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
        assert_eq!(result.stores.len(), 1);
        assert_eq!(
            result.stores[0].get("AutoCacheKey").unwrap(),
            "Search Results in System320"
        );
    }

    #[test]
    fn test_parse_property_drive() {
        let test_data = [
            0, 47, 0, 16, 183, 166, 245, 25, 0, 47, 69, 58, 92, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 116, 26, 89,
            94, 150, 223, 211, 72, 141, 103, 23, 51, 188, 238, 40, 186, 119, 44, 251, 245, 47, 14,
            22, 74, 163, 129, 62, 86, 12, 104, 188, 131, 0, 0,
        ];

        let (_, result) = parse_property_drive(&test_data).unwrap();
        assert_eq!(result.value, "E:\\");
        assert_eq!(result.shell_type, ShellType::UserPropertyView);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
    }
}
