/**
 * `ShellItems` contain metadata about the Windows Shell. They are often associated with Shellbags which track what directories a user has accessed using Explorer.exe
 * They are mainly found in the Registry, however they are also found in Shortcut and Jumplist files
 *
 * References:
 *   `https://github.com/libyal/libfwsi/blob/main/documentation/Windows%20Shell%20Item%20format.asciidoc`
 *   `https://studylib.net/doc/11116461/plumbing-the-depths--shellbags-sa-eric-r.-zimmerman`
 */
use super::{
    controlpanel::{parse_control_panel, parse_control_panel_entry},
    delegate::get_delegate_shellitem,
    directory::parse_directory,
    error::ShellItemError,
    game::parse_game,
    history::parse_history,
    mtp::{get_folder_name, get_mtp_device, get_storage_name},
    network::parse_network,
    property::{parse_property, parse_property_drive},
    root::parse_root,
    uri::parse_uri,
    variable::{
        check_beef, check_game, check_mtp_folder, check_mtp_storage, check_property, check_zip,
    },
    volume::parse_drive,
};
use crate::{
    artifacts::os::windows::shellitems::variable::parse_variable,
    utils::{
        encoding::base64_decode_standard,
        nom_helper::{Endian, nom_unsigned_one_byte, nom_unsigned_two_bytes},
    },
};
use common::windows::ShellItem;
use log::error;
use nom::{Needed, bytes::complete::take};

/// Parse a base64 encoded `ShellItem`
pub(crate) fn parse_encoded_shellitem(encoded: &str) -> Result<ShellItem, ShellItemError> {
    let result = base64_decode_standard(encoded);
    let data = match result {
        Ok(shelldata) => shelldata,
        Err(err) => {
            error!("[shellitems] Could not base64 decode data: {err:?}");
            return Err(ShellItemError::Decode);
        }
    };
    parse_shellitem(&data)
}

/// Parse the raw `ShellItem` bytes
pub(crate) fn parse_shellitem(data: &[u8]) -> Result<ShellItem, ShellItemError> {
    let result = get_shellitem(data);
    let (_, shell_item) = match result {
        Ok(results) => results,
        Err(_err) => {
            error!("[shellitems] Failed to parse ShellItem!");
            return Err(ShellItemError::ParseItem);
        }
    };

    Ok(shell_item)
}

/// Get the size of the `ShellItem` and then parse the type
pub(crate) fn get_shellitem(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let (input, size) = nom_unsigned_two_bytes(data, Endian::Le)?;

    // Size includes size itself
    let adjust_size = 2;
    if size < adjust_size {
        return Err(nom::Err::Incomplete(Needed::Unknown));
    }
    let (remaining_input, input) = take(size - adjust_size)(input)?;
    let (_, shellitem) = detect_shellitem(input)?;

    // ShellItems end with 0000
    let (end_input, end) = nom_unsigned_two_bytes(remaining_input, Endian::Le)?;
    if end != 0 {
        return Ok((remaining_input, shellitem));
    }
    Ok((end_input, shellitem))
}

/// Based on the provided bytes determine the `ShellItem` type and parse it
pub(crate) fn detect_shellitem(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let (input, item_type) = nom_unsigned_one_byte(data, Endian::Le)?;
    // Determine `ShellItem` using known IDs, signatures, and expected `ShellItem` size
    let directory_items = [0x31, 0x30, 0x32, 0x35, 0xb2];
    let drive_item = [0x2f, 0x23, 0x25, 0x29, 0x2a, 0x2e];
    let delegate = 0x74;
    let control_panel = 0x1;
    let control_panel_entry = 0x71;
    let network_items = [0xc3, 0x41, 0x42, 0x46, 0x47, 0x4c];
    let ftp = 0x61;
    let root_property = 0x1f;
    let subroot = 0x1e;
    let history = 0x69;
    let history_directory = 0x65;

    let beef0004 = [4, 0, 239, 190];
    let drive_property = 83;
    let root_size = 18;
    let beef00 = [0, 239, 190];

    // Based on `ShellItem` type parse the bytes and return generic `ShellItem` structure
    let (remaining_input, shellitem) =
        if directory_items.contains(&item_type) && check_beef(input, &beef0004) {
            parse_directory(input)?
        } else if item_type == ftp {
            parse_uri(input)?
        } else if check_zip(data) {
            parse_variable(data)?
        } else if check_mtp_storage(data) {
            get_storage_name(input)?
        } else if check_mtp_folder(data) {
            get_folder_name(input)?
        } else if check_game(input) {
            parse_game(input)?
        } else if drive_item.contains(&item_type) {
            let drive_size = 23;
            if data.len() == drive_size {
                return parse_drive(input);
            }

            if check_beef(data, &beef00) || data.len() < drive_size {
                return parse_root(input);
            }

            // If offset 3 == 16. Then this is the new Archive ShellItem format added in Windows 11
            if data.get(2).is_some_and(|b| *b == 16) {
                return parse_variable(data);
            }

            get_mtp_device(input)?
        } else if item_type == control_panel {
            parse_control_panel(input)?
        } else if item_type == control_panel_entry {
            parse_control_panel_entry(input)?
        } else if item_type == subroot {
            parse_root(input)?
        } else if item_type == delegate {
            get_delegate_shellitem(input)?
        } else if network_items.contains(&item_type) {
            parse_network(input)?
        } else if item_type == root_property
            && ((data.len() == drive_property)
                || (data.len() == root_size)
                || check_beef(input, &beef00)
                || check_property(input))
        {
            if data.len() == drive_property {
                return parse_property_drive(input);
            }

            let min_propety_size = 84;
            if input.len() > min_propety_size {
                return parse_property(input);
            }
            parse_root(input)?
        } else if item_type == history || item_type == history_directory {
            parse_history(input)?
        } else {
            parse_variable(data)?
        };

    Ok((remaining_input, shellitem))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::shellitems::items::{
        detect_shellitem, get_shellitem, parse_encoded_shellitem, parse_shellitem,
    };
    use common::windows::ShellType;

    #[test]
    fn test_parse_shellitem() {
        let test_data = [
            128, 0, 116, 0, 28, 0, 67, 70, 83, 70, 22, 0, 49, 0, 0, 0, 0, 0, 85, 79, 20, 189, 16,
            0, 115, 111, 117, 114, 99, 101, 0, 0, 0, 0, 116, 26, 89, 94, 150, 223, 211, 72, 141,
            103, 23, 51, 188, 238, 40, 186, 197, 205, 250, 223, 159, 103, 86, 65, 137, 71, 197,
            199, 107, 192, 182, 127, 62, 0, 9, 0, 4, 0, 239, 190, 85, 79, 20, 189, 85, 79, 20, 189,
            46, 0, 0, 0, 58, 63, 4, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 137,
            188, 35, 0, 115, 0, 111, 0, 117, 0, 114, 0, 99, 0, 101, 0, 0, 0, 66, 0, 0, 0,
        ];

        let result = parse_shellitem(&test_data).unwrap();
        assert_eq!(result.value, "source");
        assert_eq!(result.shell_type, ShellType::Delegate);
        assert_eq!(result.mft_sequence, 12);
        assert_eq!(result.mft_entry, 278330);
        assert_eq!(result.created, "2019-10-21T23:40:40.000Z");
        assert_eq!(result.modified, "2019-10-21T23:40:40.000Z");
        assert_eq!(result.accessed, "2019-10-21T23:40:40.000Z");
    }

    #[test]
    fn test_get_shellitem() {
        let test_data = [
            128, 0, 116, 0, 28, 0, 67, 70, 83, 70, 22, 0, 49, 0, 0, 0, 0, 0, 85, 79, 20, 189, 16,
            0, 115, 111, 117, 114, 99, 101, 0, 0, 0, 0, 116, 26, 89, 94, 150, 223, 211, 72, 141,
            103, 23, 51, 188, 238, 40, 186, 197, 205, 250, 223, 159, 103, 86, 65, 137, 71, 197,
            199, 107, 192, 182, 127, 62, 0, 9, 0, 4, 0, 239, 190, 85, 79, 20, 189, 85, 79, 20, 189,
            46, 0, 0, 0, 58, 63, 4, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 137,
            188, 35, 0, 115, 0, 111, 0, 117, 0, 114, 0, 99, 0, 101, 0, 0, 0, 66, 0, 0, 0,
        ];

        let (_, result) = get_shellitem(&test_data).unwrap();
        assert_eq!(result.value, "source");
        assert_eq!(result.shell_type, ShellType::Delegate);
        assert_eq!(result.mft_sequence, 12);
        assert_eq!(result.mft_entry, 278330);
        assert_eq!(result.created, "2019-10-21T23:40:40.000Z");
        assert_eq!(result.modified, "2019-10-21T23:40:40.000Z");
        assert_eq!(result.accessed, "2019-10-21T23:40:40.000Z");
    }

    #[test]
    fn test_detect_shellitem() {
        let test_data = [
            116, 0, 28, 0, 67, 70, 83, 70, 22, 0, 49, 0, 0, 0, 0, 0, 85, 79, 20, 189, 16, 0, 115,
            111, 117, 114, 99, 101, 0, 0, 0, 0, 116, 26, 89, 94, 150, 223, 211, 72, 141, 103, 23,
            51, 188, 238, 40, 186, 197, 205, 250, 223, 159, 103, 86, 65, 137, 71, 197, 199, 107,
            192, 182, 127, 62, 0, 9, 0, 4, 0, 239, 190, 85, 79, 20, 189, 85, 79, 20, 189, 46, 0, 0,
            0, 58, 63, 4, 0, 0, 0, 12, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 137, 188, 35,
            0, 115, 0, 111, 0, 117, 0, 114, 0, 99, 0, 101, 0, 0, 0, 66, 0, 0, 0,
        ];

        let (_, result) = detect_shellitem(&test_data).unwrap();
        assert_eq!(result.value, "source");
        assert_eq!(result.shell_type, ShellType::Delegate);
        assert_eq!(result.mft_sequence, 12);
        assert_eq!(result.mft_entry, 278330);
        assert_eq!(result.created, "2019-10-21T23:40:40.000Z");
        assert_eq!(result.modified, "2019-10-21T23:40:40.000Z");
        assert_eq!(result.accessed, "2019-10-21T23:40:40.000Z");
    }

    #[test]
    fn test_parse_encoded_shellitem() {
        let test_data = "gAB0ABwAQ0ZTRhYAMQAAAAAAVU8UvRAAc291cmNlAAAAAHQaWV6W39NIjWcXM7zuKLrFzfrfn2dWQYlHxcdrwLZ/PgAJAAQA775VTxS9VU8UvS4AAAA6PwQAAAAMAAAAAAAAAAAAAAAAAAAAibwjAHMAbwB1AHIAYwBlAAAAQgAAAA==";
        let result = parse_encoded_shellitem(&test_data).unwrap();
        assert_eq!(result.value, "source");
        assert_eq!(result.shell_type, ShellType::Delegate);
        assert_eq!(result.mft_sequence, 12);
        assert_eq!(result.mft_entry, 278330);
        assert_eq!(result.created, "2019-10-21T23:40:40.000Z");
        assert_eq!(result.modified, "2019-10-21T23:40:40.000Z");
        assert_eq!(result.accessed, "2019-10-21T23:40:40.000Z");
    }
}
