use super::filemetrics::FileMetricsVersion23;
use crate::utils::strings::extract_utf16_string;
use nom::bytes::complete::take;

/// Get all the accessed files
pub(crate) fn get_accessed_files<'a>(
    data: &'a [u8],
    metrics: &'a [FileMetricsVersion23],
    filename_offset: u32,
) -> nom::IResult<&'a [u8], Vec<String>> {
    let (input, _) = take(filename_offset)(data)?;

    let mut filenames: Vec<String> = Vec::new();
    // Loop through all metrics data and get the start (offset) and size of the filename
    for metric in metrics {
        let (filename_start, _) = take(metric.filename_offset)(input)?;
        let (_, filename) = take(metric.filename_size)(filename_start)?;

        filenames.push(extract_utf16_string(filename));
    }
    Ok((input, filenames))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::windows::prefetch::versions::version30::Version30;
    use crate::artifacts::os::windows::prefetch::{
        filemetrics::FileMetricsVersion23,
        filenames::get_accessed_files,
        header::{CompressedHeader, Header},
    };
    use crate::utils::compression::decompress_lzxpress_huffman;
    use std::{fs, path::PathBuf};

    #[test]
    fn test_get_accessed_files() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/win11/7Z.EXE-886612C8.pf");

        let buffer = fs::read(test_location).unwrap();

        let (data, header) = CompressedHeader::parse_compressed_header(&buffer).unwrap();
        assert_eq!(header.uncompressed_size, 51060);
        let huffman = 4;

        let decom_data =
            decompress_lzxpress_huffman(&mut data.to_vec(), header.uncompressed_size, huffman)
                .unwrap();
        assert_eq!(decom_data.len(), 51060);

        let (fileinfo_data, result) = Header::parse_header(&decom_data).unwrap();
        assert_eq!(result.version, 30);

        let (_, result) = Version30::parse_file_info_ver30(&fileinfo_data).unwrap();
        assert_eq!(result.file_array_offset, 296);
        assert_eq!(result.number_files, 64);

        let (_, metrics) = FileMetricsVersion23::parse_file_metrics(
            &decom_data,
            result.file_array_offset,
            &result.number_files,
        )
        .unwrap();

        let (_, filenames) =
            get_accessed_files(&decom_data, &metrics, result.filename_offset).unwrap();
        assert_eq!(filenames.len(), 64);
        assert_eq!(
            filenames[0],
            "\\VOLUME{01d6828290579d13-4290933e}\\WINDOWS\\SYSTEM32\\NTDLL.DLL"
        );
        assert_eq!(
            filenames[12],
            "\\VOLUME{01d6828290579d13-4290933e}\\WINDOWS\\SYSWOW64\\NTDLL.DLL"
        );
        assert_eq!(filenames[63], "\\VOLUME{01d6828290579d13-4290933e}\\USERS\\BOB\\APPDATA\\LOCAL\\TEMP\\CHOCOLATEY\\PSEXEC.2.40\\EULA.TXT");
    }
}
