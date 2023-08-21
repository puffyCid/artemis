use super::{
    beef::{beef0000, beef0019},
    items::ShellItem,
    property::parse_property,
};
use crate::{
    artifacts::os::windows::shellitems::{
        beef::{beef0004, beef0013, beef0026},
        items::ShellType::Variable,
    },
    utils::{
        nom_helper::{nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian},
        strings::{extract_utf16_string, extract_utf8_string},
        uuid::format_guid_le_bytes,
    },
};
use log::info;
use nom::bytes::complete::{take, take_until, take_while};
use serde_json::Value;
use std::{collections::HashMap, mem::size_of};

#[derive(PartialEq, Hash, Eq)]
enum BeefTypes {
    Beef0000,
    Beef0019,
    Beef0004,
    Beef0026,
    Beef0013,
}

/// Parse a `variable` `ShellItem`. May contain any 0xbeef00XX shell extension, zip file content, FTP URI, GUID, or Property view
pub(crate) fn parse_variable(data: &[u8]) -> nom::IResult<&[u8], ShellItem> {
    let mut variable_item = ShellItem {
        value: String::new(),
        shell_type: Variable,
        created: 0,
        modified: 0,
        accessed: 0,
        mft_entry: 0,
        mft_sequence: 0,
        stores: Vec::new(),
    };

    let sigs = get_beef_sigs();
    for (key, beef_sig) in sigs {
        let result = scan_bytes(data, &beef_sig);
        let (input, _) = match result {
            Ok(results) => results,
            Err(_err) => continue,
        };
        let beef_adjust = 4;
        let (input, _) = take(input.len() - beef_adjust)(data)?;

        let result = match key {
            BeefTypes::Beef0000 => beef0000::parse_beef(input),
            BeefTypes::Beef0019 => beef0019::parse_beef(input),
            BeefTypes::Beef0004 => return beef0004::parse_beef(input, Variable),
            BeefTypes::Beef0026 => {
                let (input, (created, modified, accessed)) = beef0026::parse_beef(input)?;
                variable_item.created = created;
                variable_item.modified = modified;
                variable_item.accessed = accessed;
                return Ok((input, variable_item));
            }
            BeefTypes::Beef0013 => beef0013::parse_beef(input),
        };
        let (input, guid) = match result {
            Ok(results) => results,
            Err(_err) => continue,
        };

        // If beef0000 returns dummy GUID try searching again
        if guid.contains("0000000-0000-0000-0000-00000") {
            return parse_variable(input);
        }

        // If we parsed an undocumented Beef extension keep searching
        if guid.is_empty() {
            continue;
        }
        variable_item.value = guid;
        return Ok((input, variable_item));
    }

    info!("[shellitem] No beef signatures found in variable item, trying Zip file contents, Propety View, and then FTP URI");
    if check_zip(data) {
        let (input, (is_zip, directory)) = parse_zip(data)?;
        if is_zip {
            variable_item.value = directory;
            return Ok((input, variable_item));
        }
    }

    if check_property(data) {
        // Nom till we get to start of property store
        let (input, _) = take(size_of::<u8>())(data)?;
        let (is_property, stores) = get_property(input);
        if is_property {
            variable_item.stores = stores;
            return Ok((input, variable_item));
        }
    }

    let (input, (is_guid, guid)) = check_guid(data)?;
    if is_guid {
        variable_item.value = guid;
        return Ok((input, variable_item));
    }
    // Try FTP variable now
    let (input, uri) = parse_ftp_uri(data)?;
    variable_item.value = uri;
    Ok((input, variable_item))
}

/// Parse a `variable` FTP URI
fn parse_ftp_uri(data: &[u8]) -> nom::IResult<&[u8], String> {
    let (input, _class_type) = take(size_of::<u8>())(data)?;
    let (input, _unknown) = take(size_of::<u8>())(input)?;
    let (input, _unknown2) = take(size_of::<u16>())(input)?;
    let (input, _unknown3) = take(size_of::<u16>())(input)?;

    let (input, _unknown4) = take(size_of::<u16>())(input)?;
    let (input, _unknown5) = take(size_of::<u32>())(input)?;
    let (input, _unknown6) = take(size_of::<u32>())(input)?;
    let (input, _unknown7) = take(size_of::<u32>())(input)?;

    let (input, _unknown_filetime) = take(size_of::<u64>())(input)?;
    let (input, _unknown8) = take(size_of::<u32>())(input)?;
    let (input, _unknown9) = take(size_of::<u32>())(input)?;

    let end_of_string = 0;
    let (input, ascii_string) = take_while(|b| b != end_of_string)(input)?;
    let uri = extract_utf8_string(ascii_string);

    // The URI is represented as ASCII and also UTF16, currently just getting ASCII URI
    Ok((input, uri))
}

/// Parse the `ZIP content` `ShellItem`.
pub(crate) fn parse_zip(data: &[u8]) -> nom::IResult<&[u8], (bool, String)> {
    // Zip file contents also do not have a dedicated id
    // However, the size of the directory name is at offset 0x54, in addition a second name may be found at offset 0x58
    let min_size = 91;
    if data.len() < min_size {
        return Ok((data, (false, String::new())));
    }
    let start_offset: u8 = 82;
    let (input, _) = take(start_offset)(data)?;
    let (input, size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let (input, size2) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let utf16_size = 2;
    if input.len() < (size * utf16_size + size2 * utf16_size) as usize {
        return Ok((data, (false, String::new())));
    }

    let (mut input, string_data) = take(size * utf16_size)(input)?;
    let mut value = extract_utf16_string(string_data);

    let empty_string = 0;
    if size2 != empty_string {
        // Nom end of string character
        let (remaining, _) = take(size_of::<u16>())(input)?;
        let (remaining, string_data) = take(size2 * utf16_size)(remaining)?;

        value = format!("{value}/{}", extract_utf16_string(string_data));
        input = remaining;
    }

    Ok((input, (true, value)))
}

/// Parse a `Variable` Property Store `ShellItem`
fn get_property(data: &[u8]) -> (bool, Vec<HashMap<String, Value>>) {
    let result = parse_property(data);
    let (_, item) = match result {
        Ok(results) => results,
        Err(_err) => {
            return (false, Vec::new());
        }
    };

    (true, item.stores)
}

/// Check if the `ShellItem` could be a ZIP type by checking for the value "/" at offsets 37 or 45
pub(crate) fn check_zip(data: &[u8]) -> bool {
    let min_size_na = 37;
    let min_size_date = 45;
    if data.len() < min_size_na || data.len() < min_size_date {
        return false;
    }

    // Zip file contents either have "N/A" or a timestamp formatted as MM/DD/YYYY
    // Check for presence of "/"
    let slash = 47;
    let adjust_vec_start = 1;
    if data[min_size_date - adjust_vec_start] == slash
        || data[min_size_na - adjust_vec_start] == slash
    {
        return true;
    }
    false
}

/// Scan for a provided `0xbeefXXXX` bytes signature
pub(crate) fn check_beef(data: &[u8], beef: &[u8]) -> bool {
    let result = scan_bytes(data, beef);
    match result {
        Ok(_result) => true,
        Err(_err) => false,
    }
}

/// Scan for Property Store bytes signature `1SPS`
pub(crate) fn check_property(data: &[u8]) -> bool {
    let prop = [49, 83, 80, 83];
    let result = scan_bytes(data, &prop);
    match result {
        Ok(_result) => true,
        Err(_err) => false,
    }
}

/// Scan for MTP storage bytes signature
pub(crate) fn check_mtp_storage(data: &[u8]) -> bool {
    let mtp = [5, 32, 49, 16];
    let result = scan_bytes(data, &mtp);
    match result {
        Ok(_result) => true,
        Err(_err) => false,
    }
}

/// Scan for Game folder bytes signature
pub(crate) fn check_game(data: &[u8]) -> bool {
    let game = [71, 70, 83, 73];
    let result = scan_bytes(data, &game);
    match result {
        Ok(_result) => true,
        Err(_err) => false,
    }
}

/// Scan for MTP folder bytes signature
pub(crate) fn check_mtp_folder(data: &[u8]) -> bool {
    let mtp = [6, 32, 25, 7];
    let result = scan_bytes(data, &mtp);
    match result {
        Ok(_result) => true,
        Err(_err) => false,
    }
}

/// Scan `ShellItem` bytes for specific bytes
fn scan_bytes<'a>(data: &'a [u8], target: &[u8]) -> nom::IResult<&'a [u8], ()> {
    let (_, scan_data) = take_until(target)(data)?;
    Ok((scan_data, ()))
}

/// Get list of supported 0xbeefXXXX signature formats
fn get_beef_sigs() -> HashMap<BeefTypes, [u8; 4]> {
    HashMap::from([
        (BeefTypes::Beef0000, [0, 0, 239, 190]),
        (BeefTypes::Beef0019, [25, 0, 239, 190]),
        (BeefTypes::Beef0004, [4, 0, 239, 190]),
        (BeefTypes::Beef0026, [38, 0, 239, 190]),
        (BeefTypes::Beef0013, [19, 0, 239, 190]),
    ])
}

/// Check if `variable` `ShellItem` contains a GUID
fn check_guid(data: &[u8]) -> nom::IResult<&[u8], (bool, String)> {
    let guid = [238, 187, 254, 35];

    let result = scan_bytes(data, &guid);
    match result {
        Ok(_result) => {}
        Err(_err) => return Ok((data, (false, String::new()))),
    }
    let min_size = 30;
    if data.len() < min_size {
        return Ok((data, (false, String::new())));
    }

    let guid_size_offset: u8 = 10;
    let (input, _) = take(guid_size_offset)(data)?;
    let (input, size) = nom_unsigned_two_bytes(input, Endian::Le)?;

    let expected_size = 16;
    if size != expected_size {
        return Ok((data, (false, String::new())));
    }

    let (input, guid_data) = take(size)(input)?;
    let guid = format_guid_le_bytes(guid_data);
    Ok((input, (true, guid)))
}

#[cfg(test)]
mod tests {
    use super::{get_beef_sigs, parse_variable, scan_bytes};
    use crate::artifacts::os::windows::shellitems::{
        items::ShellType,
        variable::{
            check_beef, check_game, check_guid, check_mtp_folder, check_mtp_storage,
            check_property, check_zip, get_property, parse_ftp_uri, parse_zip,
        },
    };

    #[test]
    fn test_parse_variable() {
        let test_data = [
            158, 0, 0, 0, 26, 0, 238, 187, 254, 35, 0, 0, 16, 0, 125, 177, 13, 123, 210, 156, 147,
            74, 151, 51, 70, 204, 137, 2, 46, 124, 0, 0, 42, 0, 0, 0, 0, 0, 239, 190, 0, 0, 0, 32,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 32,
            0, 42, 0, 0, 0, 0, 0, 239, 190, 126, 71, 179, 251, 228, 201, 59, 75, 162, 186, 211,
            245, 211, 205, 70, 249, 130, 7, 186, 130, 122, 91, 105, 69, 181, 215, 236, 131, 8, 95,
            8, 204, 32, 0, 42, 0, 0, 0, 0, 0, 239, 190, 0, 0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 32, 0, 0, 0,
        ];
        let (_, result) = parse_variable(&test_data).unwrap();
        assert_eq!(result.value, "fbb3477e-c9e4-4b3b-a2ba-d3f5d3cd46f9");
        assert_eq!(result.shell_type, ShellType::Variable);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
        assert_eq!(result.created, 0);
        assert_eq!(result.modified, 0);
        assert_eq!(result.accessed, 0);
    }

    #[test]
    fn test_parse_variable_beef0019() {
        let test_data = [
            116, 0, 0, 0, 26, 0, 238, 187, 254, 35, 0, 0, 16, 0, 125, 177, 13, 123, 210, 156, 147,
            74, 151, 51, 70, 204, 137, 2, 46, 124, 0, 0, 42, 0, 0, 0, 0, 0, 239, 190, 0, 0, 0, 32,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 32,
            0, 42, 0, 0, 0, 25, 0, 239, 190, 126, 71, 179, 251, 228, 201, 59, 75, 162, 186, 211,
            245, 211, 205, 70, 249, 130, 7, 186, 130, 122, 91, 105, 69, 181, 215, 236, 131, 8, 95,
            8, 204, 32, 0, 0, 0,
        ];
        let (_, result) = parse_variable(&test_data).unwrap();
        assert_eq!(result.value, "fbb3477e-c9e4-4b3b-a2ba-d3f5d3cd46f9");
        assert_eq!(result.shell_type, ShellType::Variable);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
        assert_eq!(result.created, 0);
        assert_eq!(result.modified, 0);
        assert_eq!(result.accessed, 0);
    }

    #[test]
    fn test_parse_variable_ftp() {
        let test_data = [
            0, 0, 0, 0, 5, 0, 3, 0, 16, 0, 0, 0, 18, 0, 0, 0, 0, 0, 0, 0, 0, 0, 28, 58, 222, 1,
            215, 1, 85, 7, 0, 0, 0, 0, 0, 0, 49, 0, 0, 0, 49, 0, 0, 0, 0, 0,
        ];
        let (_, result) = parse_variable(&test_data).unwrap();
        assert_eq!(result.value, "1");
        assert_eq!(result.shell_type, ShellType::Variable);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
        assert_eq!(result.created, 0);
        assert_eq!(result.modified, 0);
        assert_eq!(result.accessed, 0);
    }

    #[test]
    fn test_parse_ftp_uri() {
        let test_data = [
            0, 0, 0, 0, 5, 0, 3, 0, 16, 0, 0, 0, 18, 0, 0, 0, 0, 0, 0, 0, 0, 0, 28, 58, 222, 1,
            215, 1, 85, 7, 0, 0, 0, 0, 0, 0, 49, 0, 0, 0, 49, 0, 0, 0, 0, 0,
        ];
        let (_, result) = parse_ftp_uri(&test_data).unwrap();
        assert_eq!(result, "1");
    }

    #[test]
    fn test_parse_zip() {
        let test_data = [
            223, 214, 255, 127, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 16, 0, 1, 0, 48, 0, 55, 0, 47, 0, 48, 0, 53, 0, 47, 0, 50, 0, 48, 0, 49, 0,
            54, 0, 32, 0, 32, 0, 48, 0, 51, 0, 58, 0, 48, 0, 51, 0, 58, 0, 51, 0, 48, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 25, 0, 0, 0, 0, 0, 0, 0, 99, 0, 111, 0, 109, 0, 112, 0, 111, 0, 117, 0,
            110, 0, 100, 0, 102, 0, 105, 0, 108, 0, 101, 0, 114, 0, 101, 0, 97, 0, 100, 0, 101, 0,
            114, 0, 45, 0, 109, 0, 97, 0, 115, 0, 116, 0, 101, 0, 114, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0,
        ];

        let (_, (found, result)) = parse_zip(&test_data).unwrap();
        assert_eq!(found, true);
        assert_eq!(result, "compoundfilereader-master");
    }

    #[test]
    fn test_parse_variable_zip() {
        let test_data = [
            223, 214, 255, 127, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 16, 0, 1, 0, 48, 0, 55, 0, 47, 0, 48, 0, 53, 0, 47, 0, 50, 0, 48, 0, 49, 0,
            54, 0, 32, 0, 32, 0, 48, 0, 51, 0, 58, 0, 48, 0, 51, 0, 58, 0, 51, 0, 48, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 25, 0, 0, 0, 0, 0, 0, 0, 99, 0, 111, 0, 109, 0, 112, 0, 111, 0, 117, 0,
            110, 0, 100, 0, 102, 0, 105, 0, 108, 0, 101, 0, 114, 0, 101, 0, 97, 0, 100, 0, 101, 0,
            114, 0, 45, 0, 109, 0, 97, 0, 115, 0, 116, 0, 101, 0, 114, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0,
        ];

        let (_, result) = parse_variable(&test_data).unwrap();
        assert_eq!(result.value, "compoundfilereader-master");
        assert_eq!(result.shell_type, ShellType::Variable);
        assert_eq!(result.mft_sequence, 0);
        assert_eq!(result.mft_entry, 0);
        assert_eq!(result.created, 0);
        assert_eq!(result.modified, 0);
        assert_eq!(result.accessed, 0);
    }

    #[test]
    fn test_get_beef_sigs() {
        let result = get_beef_sigs();
        assert!(result.len() > 2)
    }

    #[test]
    fn test_scan_bytes() {
        let data = [2, 1, 1, 1, 0];
        let scan = [1, 1];
        let (results, _) = scan_bytes(&data, &scan).unwrap();
        assert_eq!(results, [2]);
    }

    #[test]
    fn test_check_mtp_folder() {
        let data = [6, 32, 25, 7, 0, 12, 122];
        let results = check_mtp_folder(&data);
        assert_eq!(results, true);
    }

    #[test]
    fn test_check_mtp_storage() {
        let data = [6, 32, 25, 7, 0, 12, 122, 5, 32, 49, 16];
        let results = check_mtp_storage(&data);
        assert_eq!(results, true);
    }

    #[test]
    fn test_check_property() {
        let data = [6, 32, 25, 7, 0, 49, 83, 80, 83, 12, 122, 5, 32, 49, 16];
        let results = check_property(&data);
        assert_eq!(results, true);
    }

    #[test]
    fn test_check_beef() {
        let data = [
            6, 32, 25, 7, 0, 49, 83, 80, 83, 12, 26, 0, 239, 190, 122, 5, 32, 49, 16,
        ];
        let results = check_beef(&data, &[26, 0, 239, 190]);
        assert_eq!(results, true);
    }

    #[test]
    fn test_check_guid() {
        let test_data = [
            0, 0, 26, 0, 238, 187, 254, 35, 0, 0, 16, 0, 58, 204, 191, 180, 44, 219, 76, 66, 176,
            41, 127, 233, 154, 135, 198, 65, 0, 0, 0, 0,
        ];

        let (_, (is_guid, result)) = check_guid(&test_data).unwrap();
        assert_eq!(is_guid, true);
        assert_eq!(result, "b4bfcc3a-db2c-424c-b029-7fe99a87c641");
    }

    #[test]
    fn test_check_zip() {
        let data = [
            0, 0, 63, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 16, 0, 1, 0, 48, 0, 53, 0, 47, 0, 50, 0, 55, 0, 47, 0, 50, 0, 48, 0, 49, 0, 50, 0,
            32, 0, 32, 0, 49, 0, 51, 0, 58, 0, 53, 0, 55, 0, 58, 0, 51, 0, 50, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 32, 0, 0, 0, 0, 0, 0, 0, 83, 0, 97, 0, 99, 0, 114, 0, 101, 0, 100, 0, 32, 0, 71,
            0, 111, 0, 108, 0, 100, 0, 32, 0, 38, 0, 32, 0, 83, 0, 116, 0, 111, 0, 114, 0, 109, 0,
            32, 0, 83, 0, 105, 0, 108, 0, 118, 0, 101, 0, 114, 0, 32, 0, 86, 0, 49, 0, 46, 0, 48,
            0, 53, 0, 0, 0, 0, 0, 0, 0, 0, 20, 0, 0,
        ];
        let results = check_zip(&data);
        assert_eq!(results, true);
    }

    #[test]
    fn test_get_property() {
        let data = [
            0, 10, 1, 187, 175, 147, 59, 252, 0, 4, 0, 0, 0, 0, 0, 45, 0, 0, 0, 49, 83, 80, 83,
            115, 67, 229, 10, 190, 67, 173, 79, 133, 228, 105, 220, 134, 51, 152, 110, 17, 0, 0, 0,
            11, 0, 0, 0, 0, 11, 0, 0, 0, 255, 255, 0, 0, 0, 0, 0, 0, 69, 0, 0, 0, 49, 83, 80, 83,
            48, 241, 37, 183, 239, 71, 26, 16, 165, 241, 2, 96, 140, 158, 235, 172, 41, 0, 0, 0,
            10, 0, 0, 0, 0, 31, 0, 0, 0, 12, 0, 0, 0, 118, 0, 109, 0, 119, 0, 97, 0, 114, 0, 101,
            0, 45, 0, 104, 0, 111, 0, 115, 0, 116, 0, 0, 0, 0, 0, 0, 0, 89, 0, 0, 0, 49, 83, 80,
            83, 166, 106, 99, 40, 61, 149, 210, 17, 181, 214, 0, 192, 79, 217, 24, 208, 61, 0, 0,
            0, 31, 0, 0, 0, 0, 31, 0, 0, 0, 22, 0, 0, 0, 86, 0, 77, 0, 119, 0, 97, 0, 114, 0, 101,
            0, 32, 0, 83, 0, 104, 0, 97, 0, 114, 0, 101, 0, 100, 0, 32, 0, 70, 0, 111, 0, 108, 0,
            100, 0, 101, 0, 114, 0, 115, 0, 0, 0, 0, 0, 0, 0, 45, 0, 0, 0, 49, 83, 80, 83, 58, 164,
            189, 222, 179, 55, 131, 67, 145, 231, 68, 152, 218, 41, 149, 171, 17, 0, 0, 0, 3, 0, 0,
            0, 0, 19, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];
        let (is_property, stores) = get_property(&data);
        assert_eq!(is_property, true);
        assert_eq!(stores.len(), 4);
        assert_eq!(stores[1].get("value0").unwrap(), "vmware-host");
    }

    #[test]
    fn test_check_game() {
        let data = [0, 0, 71, 70, 83, 73, 229, 134, 82, 32, 242];
        let results = check_game(&data);
        assert_eq!(results, true);
    }
}
