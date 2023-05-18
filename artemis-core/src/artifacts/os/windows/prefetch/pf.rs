use super::{
    error::PrefetchError,
    filemetrics::FileMetricsVersion23,
    filenames::get_accessed_files,
    header::{CompressedHeader, Header},
    parser::Prefetch,
    volume::Volume,
};
use crate::{
    artifacts::os::windows::prefetch::versions::version::VersionInfo,
    utils::compression::decompress_lzxpress_huffman,
};
use log::error;

impl Prefetch {
    /// Parse Prefetch files and return parsed data or error
    pub(crate) fn parse_prefetch(data: &[u8], path: &str) -> Result<Prefetch, PrefetchError> {
        let is_compressed_results = CompressedHeader::is_compressed(data);
        let is_compressed = match is_compressed_results {
            Ok((_, result)) => result,
            Err(err) => {
                error!("[prefetch] Failed to check for Prefetch compression signature: {err:?}");
                return Err(PrefetchError::Header);
            }
        };

        let (pf_data, header) = if is_compressed {
            // Parse header to get uncompressed size
            let pf_data_results = CompressedHeader::parse_compressed_header(data);
            match pf_data_results {
                Ok((pf_data, result)) => (pf_data, result),
                Err(err) => {
                    error!("[prefetch] Failed to get compressed header data: {err:?}");
                    return Err(PrefetchError::Header);
                }
            }
        } else {
            // Data is not compressed
            return Prefetch::get_prefetch_data(data, path);
        };
        let huffman = 4;

        let pf_data_result =
            decompress_lzxpress_huffman(&mut pf_data.to_vec(), header.uncompressed_size, huffman);
        let pf_data = match pf_data_result {
            Ok(result) => result,
            Err(err) => {
                error!("[prefetch] Could not decompress data: {err:?}");
                return Err(PrefetchError::Decompress);
            }
        };

        Prefetch::get_prefetch_data(&pf_data, path)
    }

    /// Get each part of the prefetch file format
    fn get_prefetch_data(data: &[u8], path: &str) -> Result<Prefetch, PrefetchError> {
        let results = Header::parse_header(data);

        let (pf_data, header) = match results {
            Ok((data, result)) => (data, result),
            Err(err) => {
                error!("[prefetch] Failed to parse header: {err:?}");
                return Err(PrefetchError::Header);
            }
        };

        let results = VersionInfo::get_version_info(pf_data, header.version);
        let version = match results {
            Ok((_, result)) => result,
            Err(err) => {
                error!("[prefetch] Failed to parse prefetch version data: {err:?}");
                return Err(PrefetchError::Version);
            }
        };

        // Version 23 supports Win7+
        let results = FileMetricsVersion23::parse_file_metrics(
            data,
            version.file_array_offset,
            &version.number_files,
        );
        let metrics = match results {
            Ok((_, result)) => result,
            Err(err) => {
                error!("[prefetch] Failed to parse file metrics: {err:?}");
                return Err(PrefetchError::FileMetrics);
            }
        };

        let results = get_accessed_files(data, &metrics, version.filename_offset);
        let filenames = match results {
            Ok((_, result)) => result,
            Err(err) => {
                error!("[prefetch] Failed to get filenames: {err:?}");
                return Err(PrefetchError::Filenames);
            }
        };

        let results = Volume::parse_volume(
            data,
            version.volume_info_offset,
            &version.number_volumes,
            header.version,
        );
        let volumes = match results {
            Ok((_, result)) => result,
            Err(err) => {
                error!("[prefetch] Failed to get volume info: {err:?}");
                return Err(PrefetchError::VolumeInfo);
            }
        };

        let mut prefetch = Prefetch {
            path: path.to_string(),
            filename: header.filename,
            hash: header.pf_hash,
            last_run_time: version.run_times.first().unwrap_or(&0).to_owned(),
            all_run_times: version.run_times,
            run_count: version.run_count,
            size: header.size,
            volume_serial: Vec::new(),
            volume_creation: Vec::new(),
            volume_path: Vec::new(),
            accessed_files_count: version.number_files,
            accessed_directories_count: 0,
            accessed_files: filenames,
            accessed_directories: Vec::new(),
        };

        // Loop through multiple volumes if needed
        for mut volume in volumes {
            prefetch
                .volume_serial
                .push(format!("{:X?}", volume.volume_serial));
            prefetch.volume_creation.push(volume.volume_creation);
            prefetch.volume_path.push(volume.volume_path);

            prefetch.accessed_directories_count += volume.number_directory_strings;
            prefetch
                .accessed_directories
                .append(&mut volume.directories);
        }

        Ok(prefetch)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        artifacts::os::windows::prefetch::header::CompressedHeader,
        utils::compression::decompress_lzxpress_huffman,
    };

    use super::Prefetch;
    use std::{fs, path::PathBuf};

    #[test]
    fn test_parse_prefetch() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/win10/_IU14D2N.TMP-136252D4.pf");

        let buffer = fs::read(&test_location).unwrap();
        let results = Prefetch::parse_prefetch(&buffer, test_location.to_str().unwrap()).unwrap();

        assert_eq!(results.path.contains("_IU14D2N.TMP-136252D4.pf"), true);
        assert_eq!(results.filename, "_IU14D2N.TMP");
        assert_eq!(results.hash, "136252D4");
        assert_eq!(results.last_run_time, 1655507964);
        assert_eq!(
            results.all_run_times,
            vec![1655507964, 1646978691, 1640232797, 1635477061, 1632023865, 1628991222]
        );
        assert_eq!(results.run_count, 6);
        assert_eq!(results.size, 153064);
        assert_eq!(results.volume_serial, vec!["D49D126F"]);
        assert_eq!(results.volume_creation, vec![1443412570]);
        assert_eq!(
            results.volume_path,
            vec!["\\VOLUME{01d0f9a19c586134-d49d126f}"]
        );
        assert_eq!(results.accessed_files_count, 146);
        assert_eq!(results.accessed_directories_count, 42);
        assert_eq!(results.accessed_files.len(), 146);
        assert_eq!(results.accessed_directories.len(), 42);

        assert_eq!(
            results.accessed_files[3],
            "\\VOLUME{01d0f9a19c586134-d49d126f}\\WINDOWS\\SYSTEM32\\KERNEL32.DLL"
        );
        assert_eq!(
            results.accessed_files[49],
            "\\VOLUME{01d0f9a19c586134-d49d126f}\\WINDOWS\\SYSWOW64\\MSCTF.DLL"
        );
        assert_eq!(
            results.accessed_files[145],
            "\\VOLUME{01d0f9a19c586134-d49d126f}\\WINDOWS\\SYSTEM32\\WIN32KFULL.SYS"
        );

        assert_eq!(
            results.accessed_directories[2],
            "\\VOLUME{01d0f9a19c586134-d49d126f}\\PROGRAMDATA\\MICROSOFT"
        );
        assert_eq!(
            results.accessed_directories[28],
            "\\VOLUME{01d0f9a19c586134-d49d126f}\\USERS\\BOB\\VIDEOS"
        );
        assert_eq!(
            results.accessed_directories[41],
            "\\VOLUME{01d0f9a19c586134-d49d126f}\\PROGRAM FILES\\GIT\\USR\\SHARE\\GTK-DOC\\HTML"
        );
    }

    #[test]
    fn test_get_prefetch_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/win81/CMD.EXE-AC113AA8.pf");

        let buffer = fs::read(&test_location).unwrap();
        let results =
            Prefetch::get_prefetch_data(&buffer, test_location.to_str().unwrap()).unwrap();

        assert_eq!(results.path.contains("CMD.EXE-AC113AA8.pf"), true);
        assert_eq!(results.filename, "CMD.EXE");
        assert_eq!(results.hash, "AC113AA8");
        assert_eq!(results.last_run_time, 1590283881);
        assert_eq!(
            results.all_run_times,
            vec![
                1590283881, 1590283755, 1590283543, 1590283090, 1590279857, 1590279632, 1590279168,
                1590277802
            ]
        );
        assert_eq!(results.run_count, 80);
        assert_eq!(results.size, 14130);
        assert_eq!(results.volume_serial, vec!["7ADCE687"]);
        assert_eq!(results.volume_creation, vec![1576558381]);
        assert_eq!(results.volume_path, vec!["\\DEVICE\\HARDDISKVOLUME2"]);
        assert_eq!(results.accessed_files_count, 28);
        assert_eq!(results.accessed_directories_count, 5);
        assert_eq!(results.accessed_files.len(), 28);
        assert_eq!(results.accessed_directories.len(), 5);

        assert_eq!(
            results.accessed_files[3],
            "\\DEVICE\\HARDDISKVOLUME2\\WINDOWS\\SYSTEM32\\WOW64CPU.DLL"
        );
        assert_eq!(
            results.accessed_files[12],
            "\\DEVICE\\HARDDISKVOLUME2\\WINDOWS\\SYSWOW64\\CMDEXT.DLL"
        );
        assert_eq!(results.accessed_files[27], "\\DEVICE\\HARDDISKVOLUME2\\USERS\\BOB\\APPDATA\\LOCAL\\TEMP\\TMP832F744F467240578F4610EC7E1C7547.EXEC.CMD");

        assert_eq!(
            results.accessed_directories[2],
            "\\DEVICE\\HARDDISKVOLUME2\\WINDOWS\\GLOBALIZATION\\SORTING"
        );
        assert_eq!(
            results.accessed_directories[3],
            "\\DEVICE\\HARDDISKVOLUME2\\WINDOWS\\SYSTEM32"
        );
        assert_eq!(
            results.accessed_directories[4],
            "\\DEVICE\\HARDDISKVOLUME2\\WINDOWS\\SYSWOW64"
        );
    }

    #[test]
    fn test_decompress_win11() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/win11/7Z.EXE-886612C8.pf");

        let buffer = fs::read(test_location).unwrap();

        let (data, header) = CompressedHeader::parse_compressed_header(&buffer).unwrap();
        assert_eq!(header.uncompressed_size, 51060);
        let huffman = 4;

        let result =
            decompress_lzxpress_huffman(&mut data.to_vec(), header.uncompressed_size, huffman)
                .unwrap();
        assert_eq!(result.len(), 51060);
    }

    #[test]
    fn test_decompress_win10() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/win10/_IU14D2N.TMP-136252D4.pf");

        let buffer = fs::read(test_location).unwrap();

        let (data, header) = CompressedHeader::parse_compressed_header(&buffer).unwrap();
        assert_eq!(header.uncompressed_size, 153064);
        let huffman = 4;

        let result =
            decompress_lzxpress_huffman(&mut data.to_vec(), header.uncompressed_size, huffman)
                .unwrap();
        assert_eq!(result.len(), 153064);
    }

    #[test]
    #[should_panic(expected = "HuffmanCompression")]
    fn test_bad_compression() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/prefetch/bad data/bad_compression.pf");

        let buffer = fs::read(test_location).unwrap();
        let huffman = 4;

        let (data, header) = CompressedHeader::parse_compressed_header(&buffer).unwrap();
        let _result =
            decompress_lzxpress_huffman(&mut data.to_vec(), header.uncompressed_size, huffman)
                .unwrap();
    }
}
