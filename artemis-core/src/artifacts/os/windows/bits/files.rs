use super::error::BitsError;
use crate::{
    filesystem::files::get_filename,
    utils::{
        encoding::base64_decode_standard,
        nom_helper::{
            nom_unsigned_eight_bytes, nom_unsigned_four_bytes, nom_unsigned_one_byte,
            nom_unsigned_sixteen_bytes, Endian,
        },
        strings::extract_utf16_string,
    },
};
use common::windows::{FileInfo, TableDump};
use nom::bytes::complete::{take, take_until};
use std::mem::size_of;

/// Loop through table rows and parse out all of the active BITS files
pub(crate) fn get_files(column_rows: &[Vec<TableDump>]) -> Result<Vec<FileInfo>, BitsError> {
    let mut files: Vec<FileInfo> = Vec::new();
    for rows in column_rows {
        let mut file = FileInfo {
            filename: String::new(),
            file_id: String::new(),
            url: String::new(),
            download_bytes_size: 0,
            trasfer_bytes_size: 0,
            full_path: String::new(),
            tmp_fullpath: String::new(),
            drive: String::new(),
            volume: String::new(),
            files_transferred: 0,
        };
        // Only two (2) columns in BITS table (as of Win11)
        for column in rows {
            if column.column_name == "Id" {
                file.file_id = column.column_data.clone();
            }

            if column.column_name == "Blob" {
                let decode_results = base64_decode_standard(&column.column_data);
                if let Ok(results) = decode_results {
                    let is_legacy = false;
                    let carve = false;
                    let _ = parse_file(&results, &mut file, is_legacy, carve);
                }
            }
        }
        files.push(file);
    }

    Ok(files)
}

/// Get file info from legacy BITS format
pub(crate) fn get_legacy_files(
    data: &[u8],
    is_legacy: bool,
    carve: bool,
) -> nom::IResult<&[u8], FileInfo> {
    let mut file = FileInfo {
        filename: String::new(),
        file_id: String::new(),
        url: String::new(),
        download_bytes_size: 0,
        trasfer_bytes_size: 0,
        full_path: String::new(),
        tmp_fullpath: String::new(),
        drive: String::new(),
        volume: String::new(),
        files_transferred: 0,
    };

    let (input, _) = parse_file(data, &mut file, is_legacy, carve)?;
    Ok((input, file))
}

/// Parse file details from data. It pretty much all strings
pub(crate) fn parse_file<'a>(
    data: &'a [u8],
    file_info: &mut FileInfo,
    is_legacy: bool,
    carve: bool,
) -> nom::IResult<&'a [u8], ()> {
    let mut input = data;

    if is_legacy {
        // This key means we have reached some of the file details
        let delimiter = [
            54, 218, 86, 119, 111, 81, 90, 67, 172, 172, 68, 162, 72, 255, 243, 77,
        ];
        let (remaining_input, _) = take_until(delimiter.as_slice())(input)?;
        let (remaining_input, _delimilter_data) =
            nom_unsigned_sixteen_bytes(remaining_input, Endian::Le)?;
        let (remaining_input, number_files) = nom_unsigned_four_bytes(remaining_input, Endian::Le)?;
        file_info.files_transferred = number_files;
        input = remaining_input;
        if number_files == 0 {
            return Ok((input, ()));
        }
    } else {
        if carve {
            let delimiter = [
                228, 207, 158, 81, 70, 217, 151, 67, 183, 62, 38, 133, 19, 5, 26, 178,
            ];
            let (file_data, _) = take_until(delimiter.as_slice())(input)?;
            let (remaining_input, _header_data) = take(size_of::<u128>())(file_data)?;

            let (_, string_size) = nom_unsigned_four_bytes(remaining_input, Endian::Le)?;
            if string_size as usize > data.len() {
                return Ok((remaining_input, ()));
            }

            input = file_data;
        }
        let (remaining_input, _header_data) = take(size_of::<u128>())(input)?;
        input = remaining_input;
    }
    let wide_char_adjust = 2;

    let (input, string_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    if string_size as usize > data.len() {
        return Ok((input, ()));
    }
    let (input, string_data) = take(string_size * wide_char_adjust)(input)?;
    file_info.full_path = extract_utf16_string(string_data);
    file_info.filename = get_filename(&file_info.full_path);

    let (input, string_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    if string_size as usize > data.len() {
        return Ok((input, ()));
    }
    let (input, string_data) = take(string_size * wide_char_adjust)(input)?;
    file_info.url = extract_utf16_string(string_data);

    let (input, string_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    if string_size as usize > data.len() {
        return Ok((input, ()));
    }
    let (input, string_data) = take(string_size * wide_char_adjust)(input)?;
    file_info.tmp_fullpath = extract_utf16_string(string_data);

    let (input, downloaded) = nom_unsigned_eight_bytes(input, Endian::Le)?;
    let (input, total) = nom_unsigned_eight_bytes(input, Endian::Le)?;

    let (input, _unknown) = nom_unsigned_one_byte(input, Endian::Le)?;

    file_info.download_bytes_size = downloaded;
    file_info.trasfer_bytes_size = total;

    // Scan until we get to the drive size. When carving BITs entries extra data may be found before the drive letter size
    let drive_size = [4, 0, 0, 0];
    let (input, _) = take_until(drive_size.as_slice())(input)?;

    let (input, string_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    if string_size as usize > data.len() {
        return Ok((input, ()));
    }
    let (input, string_data) = take(string_size * wide_char_adjust)(input)?;
    file_info.drive = extract_utf16_string(string_data);

    let (input, string_size) = nom_unsigned_four_bytes(input, Endian::Le)?;
    if string_size as usize > data.len() {
        return Ok((input, ()));
    }
    let (input, string_data) = take(string_size * wide_char_adjust)(input)?;
    file_info.volume = extract_utf16_string(string_data);

    Ok((input, ()))
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::{
            bits::files::{get_files, get_legacy_files, parse_file},
            ese::parser::grab_ese_tables,
        },
        filesystem::files::read_file,
    };
    use common::windows::FileInfo;
    use std::path::PathBuf;

    #[test]
    fn test_get_files() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests\\test_data\\windows\\ese\\win10\\qmgr.db");

        let tables = vec![String::from("Files")];
        let bits_tables = grab_ese_tables(test_location.to_str().unwrap(), &tables).unwrap();
        let files = bits_tables.get("Files").unwrap();

        let files_info = get_files(files).unwrap();
        assert_eq!(files_info.len(), 1);
    }

    #[test]
    fn test_parse_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/bits/win10/file.raw");
        let data = read_file(test_location.to_str().unwrap()).unwrap();
        let mut file = FileInfo {
            filename: String::new(),
            file_id: String::new(),
            url: String::new(),
            download_bytes_size: 0,
            trasfer_bytes_size: 0,
            full_path: String::new(),
            tmp_fullpath: String::new(),
            drive: String::new(),
            volume: String::new(),
            files_transferred: 0,
        };

        let _ = parse_file(&data, &mut file, false, false).unwrap();
        assert_eq!(
            file.filename,
            "lmelglejhemejginpboagddgdfbepgmp_372_all_ZZ_djv5ss66g7sivnpz6ljtwr2zji.crx3"
        );
        assert_eq!(file.url, "http://edgedl.me.gvt1.com/edgedl/release2/chrome_component/i73exzs4s3qvwnxxzwp6zdbtbe_372/lmelglejhemejginpboagddgdfbepgmp_372_all_ZZ_djv5ss66g7sivnpz6ljtwr2zji.crx3");

        assert_eq!(file.trasfer_bytes_size, 4782);
    }

    #[test]
    fn test_get_legacy_files() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/bits/win81/file.raw");
        let data = read_file(test_location.to_str().unwrap()).unwrap();

        let (_, results) = get_legacy_files(&data, true, false).unwrap();
        assert_eq!(results.filename, "430ce39a-f827-4af6-95ea-5dd495961bfc");
        assert_eq!(results.url, "http://msedge.b.tlu.dl.delivery.mp.microsoft.com/filestreamingservice/files/430ce39a-f827-4af6-95ea-5dd495961bfc?P1=1679310325&P2=404&P3=2&P4=gIq04pKnGxAu0CYD7Z4dp926m9RhFlNWKA0S4DwXLFl0EewHwxdBHjfUgBRBgDQ2jSNekEq%2bRjDo7DvXMwXvcA%3d%3d");

        assert_eq!(results.trasfer_bytes_size, 3226443);
    }
}
