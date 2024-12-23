use crate::utils::nom_helper::{nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian};
use crate::utils::{strings::extract_utf16_string, uuid::format_guid_le_bytes};
use common::windows::ShellItem;
use common::windows::ShellType::{Mtp, Volume};
use nom::bytes::complete::take;
use std::mem::size_of;

/// Parse a `MTP Folder` `ShellItem`
pub(crate) fn get_folder_name(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    /* MTP folder contains a lot of metadata on the device folder
     * Appears to be property arrays?
     * Contains metadata on timestamps, name, and alot of GUIDs
     *
     * Currently we just get the name
     */
    let name_size_offset: u8 = 59;
    let (input, _) = take(name_size_offset)(data)?;

    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    // Nom to name offset
    let name_offset: u8 = 8;
    let (input, _) = take(name_offset)(input)?;

    let utf16_adjust = 2;
    // Name size is in UTF16 bytes
    // A size of 5 = 10 bytes (UTF16 is 2 bytes per character)
    let (input, name_data) = take(size * utf16_adjust)(input)?;
    let name = extract_utf16_string(name_data);

    let mtp_item = ShellItem {
        value: name,
        shell_type: Mtp,
        created: String::from("1970-01-01T00:00:00.000Z"),
        modified: String::from("1970-01-01T00:00:00.000Z"),
        accessed: String::from("1970-01-01T00:00:00.000Z"),
        mft_entry: 0,
        mft_sequence: 0,
        stores: Vec::new(),
    };

    Ok((input, mtp_item))
}

/// Parse a `MTP Storage` `ShellItem`
pub(crate) fn get_storage_name(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    /* MTP Storage contains a lot of metadata on the device storage
     * Appears to be propety arrays?
     * Contains metadata on device storage size, free space, name, and alot of GUIDs
     *
     * Currently we just get the name
     */
    let name_size_offset: u8 = 35;
    let (input, _) = take(name_size_offset)(data)?;

    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;

    // Nom to name offset
    let name_offset: u8 = 12;
    let (input, _) = take(name_offset)(input)?;

    let utf16_adjust = 2;
    // Name size is in UTF16 bytes
    // A size of 5 = 10 bytes (UTF16 is 2 bytes per character)
    let (input, name_data) = take(size * utf16_adjust)(input)?;
    let name = extract_utf16_string(name_data);

    let mtp_item = ShellItem {
        value: name,
        shell_type: Mtp,
        created: String::from("1970-01-01T00:00:00.000Z"),
        modified: String::from("1970-01-01T00:00:00.000Z"),
        accessed: String::from("1970-01-01T00:00:00.000Z"),
        mft_entry: 0,
        mft_sequence: 0,
        stores: Vec::new(),
    };

    Ok((input, mtp_item))
}

/// Parse a `MTP` `ShellItem` and extract the device name and return generic `ShellItem` structure
pub(crate) fn get_mtp_device(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let (input, mtp) = parse_mtp(data)?;
    let item = ShellItem {
        value: mtp.device,
        shell_type: Volume,
        created: String::from("1970-01-01T00:00:00.000Z"),
        modified: String::from("1970-01-01T00:00:00.000Z"),
        accessed: String::from("1970-01-01T00:00:00.000Z"),
        mft_entry: 0,
        mft_sequence: 0,
        stores: Vec::new(),
    };

    Ok((input, item))
}

pub(crate) struct MtpShell {
    pub(crate) device: String,
    pub(crate) _device_path: String,
    pub(crate) _device_guid: String,
    pub(crate) _object_guid: String,
    pub(crate) _category_guid: String,
    pub(crate) _folder_guid: String,
    pub(crate) _id: String,
}

/// Parse a `MTP` `ShellItem` and return MTP `ShellItem` structure
pub(crate) fn parse_mtp(data: &[u8]) -> nom::IResult<&[u8], MtpShell> {
    let (input, _unknown) = take(size_of::<u8>())(data)?;
    let (input, size) = nom_unsigned_two_bytes(input, Endian::Le)?;

    let (remaining_input, input) = take(size)(input)?;
    let (input, _unknown_sig) = take(size_of::<u32>())(input)?;

    // Unknown data follows, might be 4 byte flags?
    let unknown_data_size: u8 = 20;
    let (input, _) = take(unknown_data_size)(input)?;

    let (input, size_device) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, _unknown2) = take(size_of::<u16>())(input)?;

    let utf16_adjust = 2;
    let (input, device_bytes) = take(size_device * utf16_adjust)(input)?;

    let (input, device_path_bytes) = take(size * utf16_adjust)(input)?;

    let device = extract_utf16_string(device_bytes);
    let device_path = extract_utf16_string(device_path_bytes);

    let (input, _unknown3) = take(size_of::<u32>())(input)?;

    // Unknown data after device path. Might be another GUID
    let (input, _) = take(unknown_data_size)(input)?;

    let (input, guid_bytes) = take(size_of::<u128>())(input)?;
    let device_guid = format_guid_le_bytes(guid_bytes);

    // Possibly two 4 byte flags?
    let (input, _unknown4) = take(size_of::<u64>())(input)?;

    // Repeat of device name
    let (input, _device_size) = take(size_of::<u32>())(input)?;
    let (input, _device_bytes) = take(size_device * utf16_adjust)(input)?;

    let (input, guid_bytes) = take(size_of::<u128>())(input)?;
    let object_guid = format_guid_le_bytes(guid_bytes);

    // Possibly two 4 byte flags?
    let (input, _unknown4) = take(size_of::<u64>())(input)?;

    let (_, guid_bytes) = take(size_of::<u128>())(input)?;
    let category_guid = format_guid_le_bytes(guid_bytes);

    let (input, guid_bytes) = take(size_of::<u128>())(remaining_input)?;
    let folder_guid = format_guid_le_bytes(guid_bytes);

    let (input, guid_bytes) = take(size_of::<u128>())(input)?;
    let id = format_guid_le_bytes(guid_bytes);

    let mtp_item = MtpShell {
        device,
        _device_path: device_path,
        _device_guid: device_guid,
        _object_guid: object_guid,
        _category_guid: category_guid,
        _folder_guid: folder_guid,
        _id: id,
    };

    Ok((input, mtp_item))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::shellitems::mtp::{
        get_folder_name, get_mtp_device, get_storage_name, parse_mtp,
    };
    use common::windows::ShellType;

    #[test]
    fn test_parse_mtp() {
        let test_data = [
            0, 52, 1, 6, 32, 49, 8, 3, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 108, 0, 0, 0, 1, 0, 0, 0,
            8, 0, 0, 0, 74, 0, 0, 0, 0, 0, 78, 0, 101, 0, 120, 0, 117, 0, 115, 0, 32, 0, 55, 0, 0,
            0, 92, 0, 92, 0, 63, 0, 92, 0, 117, 0, 115, 0, 98, 0, 35, 0, 118, 0, 105, 0, 100, 0,
            95, 0, 49, 0, 56, 0, 100, 0, 49, 0, 38, 0, 112, 0, 105, 0, 100, 0, 95, 0, 52, 0, 101,
            0, 101, 0, 49, 0, 35, 0, 48, 0, 56, 0, 55, 0, 55, 0, 48, 0, 98, 0, 101, 0, 57, 0, 35,
            0, 123, 0, 54, 0, 97, 0, 99, 0, 50, 0, 55, 0, 56, 0, 55, 0, 56, 0, 45, 0, 97, 0, 54, 0,
            102, 0, 97, 0, 45, 0, 52, 0, 49, 0, 53, 0, 53, 0, 45, 0, 98, 0, 97, 0, 56, 0, 53, 0,
            45, 0, 102, 0, 57, 0, 56, 0, 102, 0, 52, 0, 57, 0, 49, 0, 100, 0, 52, 0, 102, 0, 51, 0,
            51, 0, 125, 0, 0, 0, 13, 0, 0, 0, 3, 213, 21, 12, 23, 208, 206, 71, 144, 22, 123, 63,
            151, 135, 33, 204, 2, 0, 0, 0, 154, 151, 212, 38, 67, 230, 38, 70, 158, 43, 115, 109,
            192, 201, 47, 220, 12, 0, 0, 0, 31, 0, 0, 0, 16, 0, 0, 0, 78, 0, 101, 0, 120, 0, 117,
            0, 115, 0, 32, 0, 55, 0, 0, 0, 147, 45, 5, 143, 202, 171, 197, 79, 165, 172, 176, 29,
            244, 219, 229, 152, 2, 0, 0, 0, 72, 0, 0, 0, 107, 70, 234, 8, 164, 227, 54, 67, 161,
            243, 164, 77, 43, 92, 67, 140, 0, 0, 116, 26, 89, 94, 150, 223, 211, 72, 141, 103, 23,
            51, 188, 238, 40, 186, 60, 109, 120, 53, 117, 176, 185, 73, 136, 221, 2, 152, 118, 225,
            28, 1, 0, 0,
        ];

        let (_, result) = parse_mtp(&test_data).unwrap();
        assert_eq!(result.device, "Nexus 7");
        assert_eq!(
            result._device_path,
            "\\\\?\\usb#vid_18d1&pid_4ee1#08770be9#{6ac27878-a6fa-4155-ba85-f98f491d4f33}"
        );
        assert_eq!(result._device_guid, "26d4979a-e643-4626-9e2b-736dc0c92fdc");
        assert_eq!(result._object_guid, "8f052d93-abca-4fc5-a5ac-b01df4dbe598");
        assert_eq!(
            result._category_guid,
            "08ea466b-e3a4-4336-a1f3-a44d2b5c438c"
        );
        assert_eq!(result._folder_guid, "5e591a74-df96-48d3-8d67-1733bcee28ba");
        assert_eq!(result._id, "35786d3c-b075-49b9-88dd-029876e11c01");
    }

    #[test]
    fn test_get_mtp_device() {
        let test_data = [
            0, 52, 1, 6, 32, 49, 8, 3, 0, 0, 0, 0, 0, 0, 0, 2, 0, 0, 0, 108, 0, 0, 0, 1, 0, 0, 0,
            8, 0, 0, 0, 74, 0, 0, 0, 0, 0, 78, 0, 101, 0, 120, 0, 117, 0, 115, 0, 32, 0, 55, 0, 0,
            0, 92, 0, 92, 0, 63, 0, 92, 0, 117, 0, 115, 0, 98, 0, 35, 0, 118, 0, 105, 0, 100, 0,
            95, 0, 49, 0, 56, 0, 100, 0, 49, 0, 38, 0, 112, 0, 105, 0, 100, 0, 95, 0, 52, 0, 101,
            0, 101, 0, 49, 0, 35, 0, 48, 0, 56, 0, 55, 0, 55, 0, 48, 0, 98, 0, 101, 0, 57, 0, 35,
            0, 123, 0, 54, 0, 97, 0, 99, 0, 50, 0, 55, 0, 56, 0, 55, 0, 56, 0, 45, 0, 97, 0, 54, 0,
            102, 0, 97, 0, 45, 0, 52, 0, 49, 0, 53, 0, 53, 0, 45, 0, 98, 0, 97, 0, 56, 0, 53, 0,
            45, 0, 102, 0, 57, 0, 56, 0, 102, 0, 52, 0, 57, 0, 49, 0, 100, 0, 52, 0, 102, 0, 51, 0,
            51, 0, 125, 0, 0, 0, 13, 0, 0, 0, 3, 213, 21, 12, 23, 208, 206, 71, 144, 22, 123, 63,
            151, 135, 33, 204, 2, 0, 0, 0, 154, 151, 212, 38, 67, 230, 38, 70, 158, 43, 115, 109,
            192, 201, 47, 220, 12, 0, 0, 0, 31, 0, 0, 0, 16, 0, 0, 0, 78, 0, 101, 0, 120, 0, 117,
            0, 115, 0, 32, 0, 55, 0, 0, 0, 147, 45, 5, 143, 202, 171, 197, 79, 165, 172, 176, 29,
            244, 219, 229, 152, 2, 0, 0, 0, 72, 0, 0, 0, 107, 70, 234, 8, 164, 227, 54, 67, 161,
            243, 164, 77, 43, 92, 67, 140, 0, 0, 116, 26, 89, 94, 150, 223, 211, 72, 141, 103, 23,
            51, 188, 238, 40, 186, 60, 109, 120, 53, 117, 176, 185, 73, 136, 221, 2, 152, 118, 225,
            28, 1, 0, 0,
        ];

        let (_, result) = get_mtp_device(&test_data).unwrap();
        assert_eq!(result.value, "Nexus 7");
        assert_eq!(result.shell_type, ShellType::Volume);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
        assert_eq!(result.created, "1970-01-01T00:00:00.000Z");
        assert_eq!(result.modified, "1970-01-01T00:00:00.000Z");
        assert_eq!(result.accessed, "1970-01-01T00:00:00.000Z");
    }

    #[test]
    fn test_get_storage_name() {
        let test_data = [
            0, 150, 5, 5, 32, 49, 16, 3, 0, 0, 0, 26, 0, 32, 0, 0, 32, 119, 154, 6, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 182, 2, 0, 0, 24, 0, 0, 0, 25, 0, 0, 0, 21, 0, 0, 0, 7, 0, 0, 0, 73,
            0, 110, 0, 116, 0, 101, 0, 114, 0, 110, 0, 97, 0, 108, 0, 32, 0, 115, 0, 104, 0, 97, 0,
            114, 0, 101, 0, 100, 0, 32, 0, 115, 0, 116, 0, 111, 0, 114, 0, 97, 0, 103, 0, 101, 0,
            0, 0, 83, 0, 73, 0, 68, 0, 45, 0, 123, 0, 49, 0, 48, 0, 48, 0, 48, 0, 49, 0, 44, 0, 44,
            0, 50, 0, 56, 0, 51, 0, 54, 0, 49, 0, 51, 0, 48, 0, 50, 0, 48, 0, 49, 0, 54, 0, 125, 0,
            0, 0, 71, 0, 101, 0, 110, 0, 101, 0, 114, 0, 105, 0, 99, 0, 32, 0, 104, 0, 105, 0, 101,
            0, 114, 0, 97, 0, 114, 0, 99, 0, 104, 0, 105, 0, 99, 0, 97, 0, 108, 0, 0, 0, 123, 0,
            69, 0, 70, 0, 50, 0, 49, 0, 48, 0, 55, 0, 68, 0, 53, 0, 45, 0, 65, 0, 53, 0, 50, 0, 65,
            0, 45, 0, 52, 0, 50, 0, 52, 0, 51, 0, 45, 0, 65, 0, 50, 0, 54, 0, 66, 0, 45, 0, 54, 0,
            50, 0, 68, 0, 52, 0, 49, 0, 55, 0, 54, 0, 68, 0, 55, 0, 54, 0, 48, 0, 51, 0, 125, 0, 0,
            0, 123, 0, 52, 0, 65, 0, 68, 0, 50, 0, 67, 0, 56, 0, 53, 0, 69, 0, 45, 0, 53, 0, 69, 0,
            50, 0, 68, 0, 45, 0, 52, 0, 53, 0, 69, 0, 53, 0, 45, 0, 56, 0, 56, 0, 54, 0, 52, 0, 45,
            0, 52, 0, 70, 0, 50, 0, 50, 0, 57, 0, 69, 0, 51, 0, 67, 0, 54, 0, 67, 0, 70, 0, 48, 0,
            125, 0, 0, 0, 123, 0, 49, 0, 65, 0, 51, 0, 51, 0, 70, 0, 55, 0, 69, 0, 52, 0, 45, 0,
            65, 0, 70, 0, 49, 0, 51, 0, 45, 0, 52, 0, 56, 0, 70, 0, 53, 0, 45, 0, 57, 0, 57, 0, 52,
            0, 69, 0, 45, 0, 55, 0, 55, 0, 51, 0, 54, 0, 57, 0, 68, 0, 70, 0, 69, 0, 48, 0, 52, 0,
            65, 0, 51, 0, 125, 0, 0, 0, 123, 0, 57, 0, 50, 0, 54, 0, 49, 0, 66, 0, 48, 0, 51, 0,
            67, 0, 45, 0, 51, 0, 68, 0, 55, 0, 56, 0, 45, 0, 52, 0, 53, 0, 49, 0, 57, 0, 45, 0, 56,
            0, 53, 0, 69, 0, 51, 0, 45, 0, 48, 0, 50, 0, 67, 0, 53, 0, 69, 0, 49, 0, 70, 0, 53, 0,
            48, 0, 66, 0, 66, 0, 57, 0, 125, 0, 0, 0, 123, 0, 54, 0, 56, 0, 48, 0, 65, 0, 68, 0,
            70, 0, 53, 0, 50, 0, 45, 0, 57, 0, 53, 0, 48, 0, 65, 0, 45, 0, 52, 0, 48, 0, 52, 0, 49,
            0, 45, 0, 57, 0, 66, 0, 52, 0, 49, 0, 45, 0, 54, 0, 53, 0, 69, 0, 51, 0, 57, 0, 51, 0,
            54, 0, 52, 0, 56, 0, 49, 0, 53, 0, 53, 0, 125, 0, 0, 0, 123, 0, 50, 0, 56, 0, 68, 0,
            56, 0, 68, 0, 51, 0, 49, 0, 69, 0, 45, 0, 50, 0, 52, 0, 57, 0, 67, 0, 45, 0, 52, 0, 53,
            0, 52, 0, 69, 0, 45, 0, 65, 0, 65, 0, 66, 0, 67, 0, 45, 0, 51, 0, 52, 0, 56, 0, 56, 0,
            51, 0, 49, 0, 54, 0, 56, 0, 69, 0, 54, 0, 51, 0, 52, 0, 125, 0, 0, 0, 123, 0, 50, 0,
            55, 0, 69, 0, 50, 0, 69, 0, 51, 0, 57, 0, 50, 0, 45, 0, 65, 0, 49, 0, 49, 0, 49, 0, 45,
            0, 52, 0, 56, 0, 69, 0, 48, 0, 45, 0, 65, 0, 66, 0, 48, 0, 67, 0, 45, 0, 69, 0, 49, 0,
            55, 0, 55, 0, 48, 0, 53, 0, 65, 0, 48, 0, 53, 0, 70, 0, 56, 0, 53, 0, 125, 0, 0, 0, 13,
            0, 0, 0, 3, 213, 21, 12, 23, 208, 206, 71, 144, 22, 123, 63, 151, 135, 33, 204, 15, 0,
            0, 0, 122, 5, 163, 1, 214, 116, 128, 78, 190, 167, 220, 76, 33, 44, 229, 10, 2, 0, 0,
            0, 19, 0, 0, 0, 3, 0, 0, 0, 122, 5, 163, 1, 214, 116, 128, 78, 190, 167, 220, 76, 33,
            44, 229, 10, 3, 0, 0, 0, 31, 0, 0, 0, 42, 0, 0, 0, 71, 0, 101, 0, 110, 0, 101, 0, 114,
            0, 105, 0, 99, 0, 32, 0, 104, 0, 105, 0, 101, 0, 114, 0, 97, 0, 114, 0, 99, 0, 104, 0,
            105, 0, 99, 0, 97, 0, 108, 0, 0, 0, 122, 5, 163, 1, 214, 116, 128, 78, 190, 167, 220,
            76, 33, 44, 229, 10, 11, 0, 0, 0, 19, 0, 0, 0, 0, 0, 0, 0, 122, 5, 163, 1, 214, 116,
            128, 78, 190, 167, 220, 76, 33, 44, 229, 10, 4, 0, 0, 0, 21, 0, 0, 0, 0, 32, 119, 154,
            6, 0, 0, 0, 122, 5, 163, 1, 214, 116, 128, 78, 190, 167, 220, 76, 33, 44, 229, 10, 5,
            0, 0, 0, 21, 0, 0, 0, 0, 240, 149, 228, 1, 0, 0, 0, 122, 5, 163, 1, 214, 116, 128, 78,
            190, 167, 220, 76, 33, 44, 229, 10, 6, 0, 0, 0, 21, 0, 0, 0, 0, 0, 0, 64, 0, 0, 0, 0,
            122, 5, 163, 1, 214, 116, 128, 78, 190, 167, 220, 76, 33, 44, 229, 10, 7, 0, 0, 0, 31,
            0, 0, 0, 48, 0, 0, 0, 73, 0, 110, 0, 116, 0, 101, 0, 114, 0, 110, 0, 97, 0, 108, 0, 32,
            0, 115, 0, 104, 0, 97, 0, 114, 0, 101, 0, 100, 0, 32, 0, 115, 0, 116, 0, 111, 0, 114,
            0, 97, 0, 103, 0, 101, 0, 0, 0, 13, 73, 107, 239, 216, 92, 122, 67, 175, 252, 218, 139,
            96, 238, 74, 60, 5, 0, 0, 0, 31, 0, 0, 0, 50, 0, 0, 0, 83, 0, 73, 0, 68, 0, 45, 0, 123,
            0, 49, 0, 48, 0, 48, 0, 48, 0, 49, 0, 44, 0, 44, 0, 50, 0, 56, 0, 51, 0, 54, 0, 49, 0,
            51, 0, 48, 0, 50, 0, 48, 0, 49, 0, 54, 0, 125, 0, 0, 0, 13, 73, 107, 239, 216, 92, 122,
            67, 175, 252, 218, 139, 96, 238, 74, 60, 4, 0, 0, 0, 31, 0, 0, 0, 48, 0, 0, 0, 73, 0,
            110, 0, 116, 0, 101, 0, 114, 0, 110, 0, 97, 0, 108, 0, 32, 0, 115, 0, 104, 0, 97, 0,
            114, 0, 101, 0, 100, 0, 32, 0, 115, 0, 116, 0, 111, 0, 114, 0, 97, 0, 103, 0, 101, 0,
            0, 0, 122, 5, 163, 1, 214, 116, 128, 78, 190, 167, 220, 76, 33, 44, 229, 10, 8, 0, 0,
            0, 31, 0, 0, 0, 2, 0, 0, 0, 0, 0, 13, 73, 107, 239, 216, 92, 122, 67, 175, 252, 218,
            139, 96, 238, 74, 60, 6, 0, 0, 0, 72, 0, 0, 0, 0, 0, 1, 48, 108, 174, 4, 72, 152, 186,
            197, 123, 70, 150, 95, 231, 13, 73, 107, 239, 216, 92, 122, 67, 175, 252, 218, 139, 96,
            238, 74, 60, 26, 0, 0, 0, 11, 0, 0, 0, 0, 0, 13, 73, 107, 239, 216, 92, 122, 67, 175,
            252, 218, 139, 96, 238, 74, 60, 7, 0, 0, 0, 72, 0, 0, 0, 96, 1, 237, 153, 255, 23, 68,
            76, 157, 152, 29, 122, 111, 148, 25, 33, 147, 45, 5, 143, 202, 171, 197, 79, 165, 172,
            176, 29, 244, 219, 229, 152, 2, 0, 0, 0, 72, 0, 0, 0, 188, 91, 240, 35, 222, 21, 42,
            76, 165, 91, 169, 175, 92, 228, 18, 239, 13, 73, 107, 239, 216, 92, 122, 67, 175, 252,
            218, 139, 96, 238, 74, 60, 23, 0, 0, 0, 31, 0, 0, 0, 14, 0, 0, 0, 115, 0, 49, 0, 48, 0,
            48, 0, 48, 0, 49, 0, 0, 0, 0, 0, 0, 0,
        ];

        let (_, result) = get_storage_name(&test_data).unwrap();
        assert_eq!(result.value, "Internal shared storage");
        assert_eq!(result.shell_type, ShellType::Mtp);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
    }

    #[test]
    fn test_get_folder_name() {
        let test_data = [
            0, 232, 2, 6, 32, 25, 7, 251, 0, 0, 0, 2, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 128, 248, 12, 106, 223, 153, 214, 1, 146, 227, 226, 39, 17, 161, 224, 72,
            171, 12, 225, 119, 5, 160, 95, 133, 52, 2, 0, 0, 8, 0, 0, 0, 8, 0, 0, 0, 39, 0, 0, 0,
            112, 0, 115, 0, 50, 0, 66, 0, 73, 0, 79, 0, 83, 0, 0, 0, 112, 0, 115, 0, 50, 0, 66, 0,
            73, 0, 79, 0, 83, 0, 0, 0, 123, 0, 68, 0, 70, 0, 56, 0, 55, 0, 69, 0, 55, 0, 53, 0, 53,
            0, 45, 0, 70, 0, 70, 0, 70, 0, 70, 0, 45, 0, 70, 0, 70, 0, 70, 0, 70, 0, 45, 0, 48, 0,
            48, 0, 48, 0, 48, 0, 45, 0, 48, 0, 48, 0, 48, 0, 48, 0, 48, 0, 48, 0, 48, 0, 48, 0, 48,
            0, 48, 0, 48, 0, 48, 0, 125, 0, 0, 0, 13, 0, 0, 0, 3, 213, 21, 12, 23, 208, 206, 71,
            144, 22, 123, 63, 151, 135, 33, 204, 12, 0, 0, 0, 13, 73, 107, 239, 216, 92, 122, 67,
            175, 252, 218, 139, 96, 238, 74, 60, 2, 0, 0, 0, 31, 0, 0, 0, 8, 0, 0, 0, 111, 0, 50,
            0, 49, 0, 0, 0, 171, 253, 212, 251, 125, 152, 119, 71, 179, 249, 114, 97, 133, 169, 49,
            43, 2, 0, 0, 0, 31, 0, 0, 0, 16, 0, 0, 0, 112, 0, 115, 0, 50, 0, 66, 0, 73, 0, 79, 0,
            83, 0, 0, 0, 13, 73, 107, 239, 216, 92, 122, 67, 175, 252, 218, 139, 96, 238, 74, 60,
            19, 0, 0, 0, 7, 0, 0, 0, 21, 152, 197, 93, 122, 137, 229, 64, 13, 73, 107, 239, 216,
            92, 122, 67, 175, 252, 218, 139, 96, 238, 74, 60, 6, 0, 0, 0, 72, 0, 0, 0, 0, 0, 1, 48,
            108, 174, 4, 72, 152, 186, 197, 123, 70, 150, 95, 231, 13, 73, 107, 239, 216, 92, 122,
            67, 175, 252, 218, 139, 96, 238, 74, 60, 7, 0, 0, 0, 72, 0, 0, 0, 146, 227, 226, 39,
            17, 161, 224, 72, 171, 12, 225, 119, 5, 160, 95, 133, 13, 73, 107, 239, 216, 92, 122,
            67, 175, 252, 218, 139, 96, 238, 74, 60, 4, 0, 0, 0, 31, 0, 0, 0, 16, 0, 0, 0, 112, 0,
            115, 0, 50, 0, 66, 0, 73, 0, 79, 0, 83, 0, 0, 0, 13, 73, 107, 239, 216, 92, 122, 67,
            175, 252, 218, 139, 96, 238, 74, 60, 23, 0, 0, 0, 31, 0, 0, 0, 14, 0, 0, 0, 115, 0, 49,
            0, 48, 0, 48, 0, 48, 0, 49, 0, 0, 0, 13, 73, 107, 239, 216, 92, 122, 67, 175, 252, 218,
            139, 96, 238, 74, 60, 5, 0, 0, 0, 31, 0, 0, 0, 78, 0, 0, 0, 123, 0, 68, 0, 70, 0, 56,
            0, 55, 0, 69, 0, 55, 0, 53, 0, 53, 0, 45, 0, 70, 0, 70, 0, 70, 0, 70, 0, 45, 0, 70, 0,
            70, 0, 70, 0, 70, 0, 45, 0, 48, 0, 48, 0, 48, 0, 48, 0, 45, 0, 48, 0, 48, 0, 48, 0, 48,
            0, 48, 0, 48, 0, 48, 0, 48, 0, 48, 0, 48, 0, 48, 0, 48, 0, 125, 0, 0, 0, 13, 73, 107,
            239, 216, 92, 122, 67, 175, 252, 218, 139, 96, 238, 74, 60, 26, 0, 0, 0, 11, 0, 0, 0,
            255, 255, 88, 80, 84, 77, 206, 79, 120, 69, 149, 200, 134, 152, 169, 188, 15, 73, 3,
            220, 0, 0, 18, 0, 0, 0, 0, 0, 88, 80, 84, 77, 206, 79, 120, 69, 149, 200, 134, 152,
            169, 188, 15, 73, 78, 220, 0, 0, 31, 0, 0, 0, 32, 0, 0, 0, 50, 0, 48, 0, 50, 0, 48, 0,
            49, 0, 48, 0, 48, 0, 51, 0, 84, 0, 49, 0, 57, 0, 52, 0, 54, 0, 50, 0, 57, 0, 0, 0, 13,
            73, 107, 239, 216, 92, 122, 67, 175, 252, 218, 139, 96, 238, 74, 60, 12, 0, 0, 0, 31,
            0, 0, 0, 16, 0, 0, 0, 112, 0, 115, 0, 50, 0, 66, 0, 73, 0, 79, 0, 83, 0, 0, 0, 0, 0, 0,
            0,
        ];

        let (_, result) = get_folder_name(&test_data).unwrap();
        assert_eq!(result.value, "ps2BIOS");
        assert_eq!(result.shell_type, ShellType::Mtp);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
    }
}
