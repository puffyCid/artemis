use super::shim::ShimcacheEntry;
use crate::utils::{
    nom_helper::{
        nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_two_bytes, Endian,
    },
    strings::extract_utf16_string,
    time::filetime_to_unixepoch,
};
use log::warn;
use nom::bytes::complete::take;

/// Parse the `Shimcache` Windows 8.1 format
pub(crate) fn win8_format<'a>(
    data: &'a [u8],
    key_path: &str,
) -> nom::IResult<&'a [u8], Vec<ShimcacheEntry>> {
    // Shimcache header on Windows 8 and 8.1 is 128 bytes in size
    let header_size: u8 = 128;
    let (mut shim_data, _) = take(header_size)(data)?;

    let mut shim_vec: Vec<ShimcacheEntry> = Vec::new();
    let mut entry = 0;
    while !shim_data.is_empty() {
        let (input, sig) = nom_unsigned_four_bytes(shim_data, Endian::Le)?;

        let sig_value = 1936994352; // 00ts
        if sig != sig_value {
            warn!("[shimcache] Did not get shimcache win8 signature");
            break;
        }

        let (input, _unknown_data) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, entry_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (remaining_data, input) = take(entry_size)(input)?;
        shim_data = remaining_data;

        let (input, path_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        let empty_path = 0;
        if path_size == empty_path {
            let shim_entry = ShimcacheEntry {
                entry,
                path: String::new(),
                last_modified: 0,
                key_path: key_path.to_string(),
            };
            entry += 1;
            shim_vec.push(shim_entry);
            continue;
        }

        // Path is UTF16
        let (input, path_data) = take(path_size)(input)?;
        let (input, _insertion_flags) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _shim_flags) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (_input, last_modified) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        // Remaining part of entry data is raw binary data

        let shim_entry = ShimcacheEntry {
            entry,
            path: extract_utf16_string(path_data),
            last_modified: filetime_to_unixepoch(&last_modified),
            key_path: key_path.to_string(),
        };
        entry += 1;
        shim_vec.push(shim_entry);
    }

    Ok((shim_data, shim_vec))
}

#[cfg(test)]
mod tests {
    use super::win8_format;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_win81_format() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimcache/win8shim.raw");
        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, shim_data) = win8_format(&buffer, "test").unwrap();
        assert_eq!(shim_data.len(), 3);
        assert_eq!(shim_data[0].entry, 0);
        assert_eq!(
            shim_data[0].path,
            "SYSVOL\\Program Files\\Windows Defender\\MpCmdRun.exe"
        );
        assert_eq!(shim_data[0].last_modified, 1621323713);
        assert_eq!(shim_data[0].key_path, "test");

        assert_eq!(shim_data[2].entry, 2);
        assert_eq!(
            shim_data[1].path,
            "SYSVOL\\Program Files\\Windows Defender\\MsMpEng.exe"
        );
        assert_eq!(shim_data[2].last_modified, 1416561412);
        assert_eq!(shim_data[2].key_path, "test");
    }
}
