use crate::utils::{
    nom_helper::{
        Endian, nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_two_bytes,
    },
    strings::extract_utf16_string,
    time::filetime_to_iso,
};
use common::windows::ShimcacheEntry;
use nom::bytes::complete::take;
use tracing::{debug, info, warn};

/// Parse the `Shimcache` Windows 10 format
pub(crate) fn win10_format<'a>(
    data: &'a [u8],
    key_path: &str,
    path: &str,
) -> nom::IResult<&'a [u8], Vec<ShimcacheEntry>> {
    info!("Shimcache Windows 10/11: {path}");
    let (_, header_size) = nom_unsigned_four_bytes(data, Endian::Le)?;

    // Windows 10 versions before the Creator update have a header size of 48 bytes
    // Versions after the Creator update have a header size of 52 bytes
    // Header does not contain anything needed to parse the shimcache data
    let (mut shim_data, _) = take(header_size)(data)?;

    let mut shim_vec: Vec<ShimcacheEntry> = Vec::new();
    let mut entry = 0;
    while !shim_data.is_empty() {
        let (input, sig) = nom_unsigned_four_bytes(shim_data, Endian::Le)?;

        let sig_value = 1936994353; // 10ts
        if sig != sig_value {
            warn!("Did not get shimcache win10 signature. Got {sig}");
            break;
        }

        let (input, _unknown_data) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let (input, entry_size) = nom_unsigned_four_bytes(input, Endian::Le)?;

        let (remaining_data, input) = take(entry_size)(input)?;
        shim_data = remaining_data;

        let (input, path_size) = nom_unsigned_two_bytes(input, Endian::Le)?;
        debug!(
            "Path size is {path_size} bytes for remaining {} bytes",
            input.len()
        );
        // Path is UTF16
        let (input, path_data) = take(path_size)(input)?;
        let (remaining, last_modified) = nom_unsigned_eight_bytes(input, Endian::Le)?;

        // Remaining part of entry data is raw binary data
        debug!("Remaining binary bytes {}", remaining.len());

        let shim_entry = ShimcacheEntry {
            entry,
            path: extract_utf16_string(path_data),
            last_modified: filetime_to_iso(last_modified),
            key_path: key_path.to_string(),
            evidence: path.to_string(),
        };
        entry += 1;
        shim_vec.push(shim_entry);
    }

    Ok((shim_data, shim_vec))
}

#[cfg(test)]
mod tests {
    use super::win10_format;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_win10_format() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimcache/win10Creatorsshim.raw");
        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, shim_data) = win10_format(&buffer, "test", "path\\SYSTEM").unwrap();
        assert_eq!(shim_data.len(), 3);
        assert_eq!(shim_data[0].entry, 0);
        assert_eq!(
            shim_data[0].path,
            "C:\\Users\\bob\\Documents\\ShellBagsExplorer\\ShellBagsExplorer.exe"
        );
        assert_eq!(shim_data[0].last_modified, "2021-01-31T02:41:02.943Z");
        assert_eq!(shim_data[0].key_path, "test");

        assert_eq!(shim_data[1].entry, 1);
        assert_eq!(
            shim_data[1].path,
            "C:\\Users\\bob\\Documents\\ShellBagsExplorer\\SBECmd.exe"
        );
        assert_eq!(shim_data[1].last_modified, "2021-01-31T02:41:00.731Z");
        assert_eq!(shim_data[1].key_path, "test");
    }
}
