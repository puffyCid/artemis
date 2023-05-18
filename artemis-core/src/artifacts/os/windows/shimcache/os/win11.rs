use super::{shim::ShimcacheEntry, win10::win10_format};

/// Windows 11 `Shimcache` is the same as Windows 10 Creators format
pub(crate) fn win11_format<'a>(
    data: &'a [u8],
    key_path: &str,
) -> nom::IResult<&'a [u8], Vec<ShimcacheEntry>> {
    win10_format(data, key_path)
}

#[cfg(test)]
mod tests {
    use super::win11_format;
    use crate::filesystem::files::read_file;
    use std::path::PathBuf;

    #[test]
    fn test_win11_format() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/shimcache/win11shim.raw");
        let buffer = read_file(&test_location.display().to_string()).unwrap();
        let (_, shim_data) = win11_format(&buffer, "test").unwrap();
        assert_eq!(shim_data.len(), 2);
        assert_eq!(shim_data[0].entry, 0);
        assert_eq!(
            shim_data[0].path,
            "C:\\WINDOWS\\system32\\wbem\\WmiApSrv.exe"
        );
        assert_eq!(shim_data[0].last_modified, 1655591210);
        assert_eq!(shim_data[0].key_path, "test");

        assert_eq!(shim_data[1].entry, 1);
        assert_eq!(
            shim_data[1].path,
            "C:\\Program Files (x86)\\Microsoft\\Edge\\Application\\msedge.exe"
        );
        assert_eq!(shim_data[1].last_modified, 1671129486);
        assert_eq!(shim_data[1].key_path, "test");
    }
}
