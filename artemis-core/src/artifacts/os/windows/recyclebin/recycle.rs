use crate::{
    filesystem::files::get_filename,
    utils::{
        nom_helper::{nom_unsigned_eight_bytes, nom_unsigned_four_bytes, Endian},
        strings::extract_utf16_string,
        time::filetime_to_unixepoch,
    },
};
use common::windows::RecycleBin;
use log::error;
use nom::{
    bytes::complete::{take, take_until},
    Needed,
};
use std::path::Path;

/// Parse the `$I` file data from the `Recycle Bin`
pub(crate) fn parse_recycle_bin(data: &[u8]) -> nom::IResult<&[u8], RecycleBin> {
    let (input, version) = nom_unsigned_eight_bytes(data, Endian::Le)?;

    let (input, size) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, deletion) = nom_unsigned_eight_bytes(input, Endian::Le)?;

    let full_path = if version == 1 {
        let (_, name_data) = take_until([0, 0].as_slice())(input)?;
        extract_utf16_string(name_data)
    } else if version == 2 {
        let (input, name_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
        let utf_adjust = 2;
        let (_, name_data) = take(name_size * utf_adjust)(input)?;
        extract_utf16_string(name_data)
    } else {
        error!("[recyclebin] Got unknown recycle bin version: {version}");
        return Err(nom::Err::Incomplete(Needed::Unknown));
    };

    let mut recycle = RecycleBin {
        size,
        deleted: filetime_to_unixepoch(&deletion),
        filename: get_filename(&full_path),
        directory: String::new(),
        full_path,
        sid: String::new(),
        recycle_path: String::new(),
    };

    let dir = Path::new(&recycle.full_path).parent();

    if let Some(path) = dir {
        recycle.directory = path.to_str().unwrap_or_default().to_string();
    }

    Ok((input, recycle))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::recyclebin::recycle::parse_recycle_bin;

    #[test]
    fn test_parse_recycle_bin() {
        let test = [
            2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 240, 31, 68, 108, 17, 165, 215, 1, 72,
            0, 0, 0, 67, 0, 58, 0, 92, 0, 85, 0, 115, 0, 101, 0, 114, 0, 115, 0, 92, 0, 98, 0, 111,
            0, 98, 0, 92, 0, 80, 0, 114, 0, 111, 0, 106, 0, 101, 0, 99, 0, 116, 0, 115, 0, 92, 0,
            111, 0, 115, 0, 113, 0, 117, 0, 101, 0, 114, 0, 121, 0, 92, 0, 98, 0, 117, 0, 105, 0,
            108, 0, 100, 0, 92, 0, 110, 0, 115, 0, 95, 0, 111, 0, 115, 0, 113, 0, 117, 0, 101, 0,
            114, 0, 121, 0, 95, 0, 117, 0, 116, 0, 105, 0, 108, 0, 115, 0, 95, 0, 115, 0, 121, 0,
            115, 0, 116, 0, 101, 0, 109, 0, 95, 0, 115, 0, 121, 0, 115, 0, 116, 0, 101, 0, 109, 0,
            117, 0, 116, 0, 105, 0, 108, 0, 115, 0, 0, 0,
        ];
        let (_, result) = parse_recycle_bin(&test).unwrap();

        assert_eq!(result.deleted, 1631147228);
        assert_eq!(result.size, 0);
        assert_eq!(result.filename, "ns_osquery_utils_system_systemutils");
        assert_eq!(
            result.full_path,
            "C:\\Users\\bob\\Projects\\osquery\\build\\ns_osquery_utils_system_systemutils"
        );
        assert_eq!(result.directory, "C:\\Users\\bob\\Projects\\osquery\\build");
    }
}
