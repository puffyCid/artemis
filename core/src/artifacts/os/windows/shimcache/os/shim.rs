use super::{win10::win10_format, win7::win7_format, win8::win8_format, win81::win81_format};
use crate::{
    artifacts::os::windows::shimcache::error::ShimcacheError,
    utils::{
        encoding::base64_decode_standard,
        nom_helper::{nom_unsigned_four_bytes, Endian},
    },
};
use common::windows::ShimcacheEntry;
use log::error;
use nom::{bytes::complete::take, error::ErrorKind};

/// Parse Windows `Shimcache` data from the Registry
pub(crate) fn parse_shimdata(
    shim_data: &str,
    key_path: &str,
    path: &str,
) -> Result<Vec<ShimcacheEntry>, ShimcacheError> {
    let binary_result = base64_decode_standard(shim_data);
    let binary_data = match binary_result {
        Ok(result) => result,
        Err(err) => {
            error!("[shimcache] Could not base64 decode shimcache: {err:?}");
            return Err(ShimcacheError::Base64);
        }
    };

    let entries_result = detect_format(&binary_data, key_path, path);
    match entries_result {
        Ok((_, result)) => Ok(result),
        Err(_err) => {
            error!("[shimcache] Could not parse Shimcache format");
            Err(ShimcacheError::Parser)
        }
    }
}

/// Try to detect the `Shimcache` format
fn detect_format<'a>(
    data: &'a [u8],
    key_path: &str,
    path: &str,
) -> nom::IResult<&'a [u8], Vec<ShimcacheEntry>> {
    let (_, sig) = nom_unsigned_four_bytes(data, Endian::Le)?;

    let win7_sig = 0xbadc0fee;
    let win8_81_size = 128;
    let win10_size = 48;
    let win10_creator_size = 52;

    let entries_result = if win7_sig == sig {
        win7_format(data, key_path, path)
    } else if win8_81_size == sig {
        let (input, _) = take(win8_81_size)(data)?;
        let (_, entry_sig) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let win8_entry = 1936994352;
        let win81_entry = 1936994353;

        if entry_sig == win8_entry {
            win8_format(data, key_path, path)
        } else if entry_sig == win81_entry {
            win81_format(data, key_path, path)
        } else {
            error!("[shimcache] Unknown Shimcache Win8 entrytype. Sig is: {entry_sig}");
            return Err(nom::Err::Failure(nom::error::Error::new(
                &[],
                ErrorKind::Fail,
            )));
        }
    } else if win10_size == sig || win10_creator_size == sig {
        win10_format(data, key_path, path)
    } else {
        error!("[shimcache] Unknown Shimcache type. Sig is: {sig}");
        return Err(nom::Err::Failure(nom::error::Error::new(
            &[],
            ErrorKind::Fail,
        )));
    };

    entries_result
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::shimcache::os::shim::{detect_format, parse_shimdata},
        utils::encoding::base64_decode_standard,
    };

    #[test]
    #[cfg(target_os = "windows")]
    fn test_parse_shimdata() {
        use crate::artifacts::os::windows::shimcache::registry::get_shimcache_data;

        let result = get_shimcache_data("C:\\Windows\\System32\\config\\SYSTEM").unwrap();
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
        let shim_data = parse_shimdata(test_data, path, "test\\test").unwrap();
        assert_eq!(shim_data.len(), 2);
        assert_eq!(shim_data[0].entry, 0);
        assert_eq!(
            shim_data[0].path,
            "C:\\WINDOWS\\system32\\wbem\\WmiApSrv.exe"
        );
        assert_eq!(shim_data[0].last_modified, "2022-06-18T22:26:50.000Z");
        assert_eq!(shim_data[0].key_path, "test");
    }

    #[test]
    fn test_detect_format() {
        let test_data = "NAAAAEoCAAAAAAAASAAAAAUAAAAAAAAAAAAAAEEBAAAAAAAAAAAAACMAAAAjAAAAAAAAADEwdHNzXgQ34AAAAEoAQwA6AFwAVwBJAE4ARABPAFcAUwBcAHMAeQBzAHQAZQBtADMAMgBcAHcAYgBlAG0AXABXAG0AaQBBAHAAUwByAHYALgBlAHgAZQAFAUyBYoPYAYgAAAAAAgAABAAAAAEAAAABAGSGAAAAAAIAAAAAAAAACAAAAAgAAAAAAAAAAAAAAAQAAAAIAAAAAAAAAAAAAAAQAAAACAAAAAAAAAAAAAAAACAAAAIAAABkhgAAAAgAAAIAAABkhgAAAAQAAAQAAAAAAAAAQAAAAAQAAAAAAAAAIAAAAAQAAAAAAAAAMTB0c+SeJT/qAAAAeABDADoAXABQAHIAbwBnAHIAYQBtACAARgBpAGwAZQBzACAAKAB4ADgANgApAFwATQBpAGMAcgBvAHMAbwBmAHQAXABFAGQAZwBlAFwAQQBwAHAAbABpAGMAYQB0AGkAbwBuAFwAbQBzAGUAZABnAGUALgBlAHgAZQA/dJdftBDZAWQAAAAAAgAABAAAAAAAAAABAGSGAAAAAAIAAAAAAAAACAAAAAgAAAAAAAAAAAAAAAQAAAAIAAAAAAAAAAAAAAAQAAAACAAAAAAAAAAAAAAAACAAAAIAAABkhgAAAAgAAAIAAABkhgAA";
        let path = "test";
        let (_, shim_data) = detect_format(
            &base64_decode_standard(test_data).unwrap(),
            path,
            "path/SYSTEM",
        )
        .unwrap();
        assert_eq!(shim_data.len(), 2);
        assert_eq!(shim_data[0].entry, 0);
        assert_eq!(
            shim_data[0].path,
            "C:\\WINDOWS\\system32\\wbem\\WmiApSrv.exe"
        );
        assert_eq!(shim_data[0].last_modified, "2022-06-18T22:26:50.000Z");
        assert_eq!(shim_data[0].key_path, "test");
    }
}
