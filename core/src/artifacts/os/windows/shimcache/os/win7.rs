use crate::utils::{
    nom_helper::{
        Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_two_bytes,
    },
    strings::extract_utf16_string,
    time::{filetime_to_unixepoch, unixepoch_to_iso},
};
use common::windows::ShimcacheEntry;
use nom::bytes::complete::take;

/// Parse Windows 7 `Shimcache` format. Depending on architecture the format is slightly different
pub(crate) fn win7_format<'a>(
    data: &'a [u8],
    key_path: &str,
    path: &str,
) -> nom::IResult<&'a [u8], Vec<ShimcacheEntry>> {
    let (input, _sig) = nom_unsigned_four_bytes(data, Endian::Le)?;
    let (input, entries) = nom_unsigned_four_bytes(input, Endian::Le)?;

    let (input, _unknown) = nom_unsigned_four_bytes(input, Endian::Le)?;
    let unknown2_size: u8 = 116;
    let (mut shim_data, _unknown2) = take(unknown2_size)(input)?;
    let mut entry = 0;
    let mut shim_vec: Vec<ShimcacheEntry> = Vec::new();

    while entry < entries {
        let (input, _path_size) = nom_unsigned_two_bytes(shim_data, Endian::Le)?;
        let (shim_input, max_path_size) = nom_unsigned_two_bytes(input, Endian::Le)?;

        // Assume 64-bit Shimcace be default
        let (input, _padding) = nom_unsigned_four_bytes(shim_input, Endian::Le)?;
        let (input, offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        // Its probably 64-bit if the offset is less than length
        if offset as usize <= data.len() {
            let (input, last_modified) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (input, _insertion_flags) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, _shim_flags) = nom_unsigned_four_bytes(input, Endian::Le)?;
            let (input, _data_size) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            let (input, _data_offset) = nom_unsigned_eight_bytes(input, Endian::Le)?;
            // Rest of format is raw binary data

            shim_data = input;
            let empty_path = 0;
            if max_path_size == empty_path {
                let shim_entry = ShimcacheEntry {
                    entry,
                    path: String::new(),
                    last_modified: String::new(),
                    key_path: key_path.to_string(),
                    source_path: path.to_string(),
                };
                entry += 1;
                shim_vec.push(shim_entry);
                continue;
            }
            // Offset is from start of Shimcache data
            let (path_start, _) = take(offset)(data)?;
            let (_, path_data) = take(max_path_size)(path_start)?;

            let shim_entry = ShimcacheEntry {
                entry,
                path: extract_utf16_string(path_data),
                last_modified: unixepoch_to_iso(&filetime_to_unixepoch(&last_modified)),
                key_path: key_path.to_string(),
                source_path: path.to_string(),
            };
            entry += 1;
            shim_vec.push(shim_entry);
            continue;
        }

        // Otherwise assume 32-bit Shimcache
        let (input, offset) = nom_unsigned_four_bytes(shim_input, Endian::Le)?;

        let (input, last_modified) = nom_unsigned_eight_bytes(input, Endian::Le)?;
        let (input, _insertion_flags) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _shim_flags) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _data_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, _data_offset) = nom_unsigned_four_bytes(input, Endian::Le)?;
        // Rest of format is raw binary data

        shim_data = input;

        let empty_path = 0;
        if max_path_size == empty_path {
            let shim_entry = ShimcacheEntry {
                entry,
                path: String::new(),
                last_modified: String::new(),
                key_path: key_path.to_string(),
                source_path: path.to_string(),
            };
            entry += 1;
            shim_vec.push(shim_entry);
            continue;
        }
        // Offset is from start of Shimcache data
        let (path_start, _) = take(offset)(data)?;
        let (_, path_data) = take(max_path_size)(path_start)?;

        let shim_entry = ShimcacheEntry {
            entry,
            path: extract_utf16_string(path_data),
            last_modified: unixepoch_to_iso(&filetime_to_unixepoch(&last_modified)),
            key_path: key_path.to_string(),
            source_path: path.to_string(),
        };
        entry += 1;
        shim_vec.push(shim_entry);
    }
    Ok((input, shim_vec))
}

#[cfg(test)]
mod tests {
    use super::win7_format;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_win7_32bit_format() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/dfir/windows/shimcache/win7/win7x86.bin");
        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, shim_data) = win7_format(&buffer, "test", "test/test").unwrap();
        assert_eq!(shim_data.len(), 91);
        assert_eq!(
            shim_data[34].path,
            "\\??\\C:\\Windows\\system32\\aitagent.EXE"
        );
        assert_eq!(shim_data[34].last_modified, "2009-07-14T01:14:11.000Z");
    }

    #[test]
    #[cfg(target_arch = "x86_64")]
    fn test_win7_64bit_format() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/dfir/windows/shimcache/win7/win7x64.bin");
        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, shim_data) = win7_format(&buffer, "test", "test/test").unwrap();
        assert_eq!(shim_data.len(), 304);
        assert_eq!(
            shim_data[34].path,
            "\\??\\C:\\Program Files (x86)\\Google\\Update\\Install\\{1632C2A5-255B-443C-9881-CB9AD5A6F24C}\\GoogleUpdateSetup.exe"
        );
        assert_eq!(shim_data[34].last_modified, "2015-01-28T21:47:00.000Z");
    }
}
