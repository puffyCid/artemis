use super::{
    win10::win10_format, win11::win11_format, win7::win7_format, win8::win8_format,
    win81::win81_format,
};
use crate::{
    artifacts::os::{systeminfo::info::get_os_version, windows::shimcache::error::ShimcacheError},
    utils::encoding::base64_decode_standard,
};
use common::windows::ShimcacheEntry;
use log::error;

pub(crate) fn parse_shimdata(
    shim_data: &str,
    key_path: &str,
) -> Result<Vec<ShimcacheEntry>, ShimcacheError> {
    let binary_result = base64_decode_standard(shim_data);
    let binary_data = match binary_result {
        Ok(result) => result,
        Err(err) => {
            error!("[shimcache] Could not base64 decode shimcache: {err:?}");
            return Err(ShimcacheError::Base64);
        }
    };

    let os = get_os_version();
    let entries_result = if os.starts_with("11") {
        win11_format(&binary_data, key_path)
    } else if os.starts_with("10") {
        win10_format(&binary_data, key_path)
    } else if os.contains("9600") {
        win81_format(&binary_data, key_path)
    } else if os.contains("9200") {
        win8_format(&binary_data, key_path)
    } else if os.contains("7601") {
        win7_format(&binary_data, key_path)
    } else {
        error!("[shimcache] Unknown Windows OS ({os}), cannot determine Shimcache format");
        return Err(ShimcacheError::UnknownOS);
    };

    match entries_result {
        Ok((_, result)) => Ok(result),
        Err(_err) => {
            error!("[shimcache] Could not parse Shimcache format");
            Err(ShimcacheError::Parser)
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::shimcache::{
        os::shim::parse_shimdata, registry::get_shimcache_data,
    };

    #[test]
    fn test_parse_shimdata() {
        let result = get_shimcache_data(&'C').unwrap();
        assert!(result.len() > 0);

        for entry in result {
            let shim_data = parse_shimdata(&entry.shim_data, &entry.key_path).unwrap();
            assert!(shim_data.len() > 3);
        }
    }

    #[test]
    fn test_parse_shimdata_win11() {
        let test_data = "NAAAAEoCAAAAAAAASAAAAAUAAAAAAAAAAAAAAEEBAAAAAAAAAAAAACMAAAAjAAAAAAAAADEwdHNzXgQ34AAAAEoAQwA6AFwAVwBJAE4ARABPAFcAUwBcAHMAeQBzAHQAZQBtADMAMgBcAHcAYgBlAG0AXABXAG0AaQBBAHAAUwByAHYALgBlAHgAZQAFAUyBYoPYAYgAAAAAAgAABAAAAAEAAAABAGSGAAAAAAIAAAAAAAAACAAAAAgAAAAAAAAAAAAAAAQAAAAIAAAAAAAAAAAAAAAQAAAACAAAAAAAAAAAAAAAACAAAAIAAABkhgAAAAgAAAIAAABkhgAAAAQAAAQAAAAAAAAAQAAAAAQAAAAAAAAAIAAAAAQAAAAAAAAAMTB0c+SeJT/qAAAAeABDADoAXABQAHIAbwBnAHIAYQBtACAARgBpAGwAZQBzACAAKAB4ADgANgApAFwATQBpAGMAcgBvAHMAbwBmAHQAXABFAGQAZwBlAFwAQQBwAHAAbABpAGMAYQB0AGkAbwBuAFwAbQBzAGUAZABnAGUALgBlAHgAZQA/dJdftBDZAWQAAAAAAgAABAAAAAAAAAABAGSGAAAAAAIAAAAAAAAACAAAAAgAAAAAAAAAAAAAAAQAAAAIAAAAAAAAAAAAAAAQAAAACAAAAAAAAAAAAAAAACAAAAIAAABkhgAAAAgAAAIAAABkhgAA";
        let path = "test";
        let shim_data = parse_shimdata(test_data, path).unwrap();
        assert_eq!(shim_data.len(), 2);
        assert_eq!(shim_data[0].entry, 0);
        assert_eq!(
            shim_data[0].path,
            "C:\\WINDOWS\\system32\\wbem\\WmiApSrv.exe"
        );
        assert_eq!(shim_data[0].last_modified, 1655591210);
        assert_eq!(shim_data[0].key_path, "test");
    }
}
