use super::error::ArtemisError;
use crate::filesystem::files::read_file;
use flate2::{write::GzEncoder, Compression};
use log::{error, warn};
use std::{
    fs::File,
    io::{copy, BufReader, Write},
};
use walkdir::WalkDir;
use zip::{write::FileOptions, ZipWriter};

#[cfg(target_os = "macos")]
/// Decompress gzip compressed file
pub(crate) fn decompress_gzip(path: &str) -> Result<Vec<u8>, ArtemisError> {
    use flate2::bufread::MultiGzDecoder;
    use std::io::Read;

    let buffer_result = read_file(path);
    let buffer = match buffer_result {
        Ok(result) => result,
        Err(err) => {
            error!("[compression] Could not read file {path}: {err:?}");
            return Err(ArtemisError::GzipReadFile);
        }
    };
    let mut data = MultiGzDecoder::new(&buffer[..]);

    let mut decompress_data = Vec::new();
    let result = data.read_to_end(&mut decompress_data);
    match result {
        Ok(_) => {}
        Err(err) => {
            error!("[compression] Could not decompress file {path}: {err:?}");
            return Err(ArtemisError::GzipDecompress);
        }
    }

    Ok(decompress_data)
}

#[cfg(target_os = "linux")]
/// Decompress zstd data
pub(crate) fn decompress_zstd(data: &[u8]) -> Result<Vec<u8>, ArtemisError> {
    use ruzstd::StreamingDecoder;
    use std::io::Read;

    let decoder_result = StreamingDecoder::new(data);
    let mut decoder = match decoder_result {
        Ok(result) => result,
        Err(err) => {
            error!("[compresssion] Could not decompress zstd data: {err:?}");
            return Err(ArtemisError::ZstdDecompresss);
        }
    };
    let mut data = Vec::new();
    decoder.read_to_end(&mut data).unwrap();
    Ok(data)
}

/// Compress a file at provided path using gzip compression
pub(crate) fn compress_gzip(path: &str) -> Result<(), ArtemisError> {
    let open_result = File::open(path);
    let target = match open_result {
        Ok(result) => result,
        Err(err) => {
            error!("[compression] Could not open file for compressing: {err:?}");
            return Err(ArtemisError::GzipOpen);
        }
    };
    let mut input = BufReader::new(target);
    let output_result = File::create(format!("{path}.gz"));
    let output = match output_result {
        Ok(result) => result,
        Err(err) => {
            error!("[compression] Could not create compressed file: {err:?}");
            return Err(ArtemisError::CompressCreate);
        }
    };
    let mut data = GzEncoder::new(output, Compression::default());

    let copy_result = copy(&mut input, &mut data);
    match copy_result {
        Ok(_) => {}
        Err(err) => {
            error!("[compression] Could not copy data to compressed file: {err:?}");
            return Err(ArtemisError::GzipCopy);
        }
    }

    let finish_status = data.finish();
    match finish_status {
        Ok(_) => {}
        Err(err) => {
            error!("[compression] Could not finish compressing data to file: {err:?}");
            return Err(ArtemisError::GzipFinish);
        }
    }
    Ok(())
}

/// Compress provided data with GZIP
pub(crate) fn compress_gzip_data(data: &[u8]) -> Result<Vec<u8>, ArtemisError> {
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    let status = gz.write_all(data);
    match status {
        Ok(_) => {}
        Err(err) => {
            error!("[compression] Could not compress data with gzip: {err:?}");
            return Err(ArtemisError::CompressCreate);
        }
    }
    let finish_status = gz.finish();

    let data = match finish_status {
        Ok(results) => results,
        Err(err) => {
            error!("[compression] Could not finish gzip compressing data: {err:?}");
            return Err(ArtemisError::GzipFinish);
        }
    };
    Ok(data)
}

/// Compress the output directory to a zip file
pub(crate) fn compress_output_zip(directory: &str, zip_name: &str) -> Result<(), ArtemisError> {
    let output_files = WalkDir::new(directory);

    let zip_file_result = File::create(format!("{zip_name}.zip"));
    let zip_file = match zip_file_result {
        Ok(result) => result,
        Err(err) => {
            error!("[compression] Could not create compressed zip: {err:?}");
            return Err(ArtemisError::CompressCreate);
        }
    };
    let options = FileOptions::default().compression_method(zip::CompressionMethod::Stored);
    let mut zip_writer = ZipWriter::new(zip_file);
    for entries in output_files {
        let entry = match entries {
            Ok(result) => result,
            Err(err) => {
                warn!("[compression] Failed to get output file info: {err:?}");
                continue;
            }
        };
        if !entry.path().is_file() {
            continue;
        }

        let name_result = entry.file_name().to_str();
        let name = if let Some(result) = name_result {
            result
        } else {
            warn!("[compression] Failed to get target filename");
            continue;
        };

        let start_result = zip_writer.start_file(name, options);
        match start_result {
            Ok(_) => {}
            Err(err) => {
                warn!("[compression] Could not start file to zip: {err:?}");
                continue;
            }
        }

        let path_result = entry.path().to_str();
        let path = if let Some(result) = path_result {
            result
        } else {
            warn!("[compression] Failed to get target path");
            continue;
        };

        let bytes_result = read_file(path);
        let bytes = match bytes_result {
            Ok(result) => result,
            Err(err) => {
                warn!("[compression] Could not read file {path}: {err:?}");
                continue;
            }
        };
        let write_result = zip_writer.write_all(&bytes);
        match write_result {
            Ok(_) => {}
            Err(err) => {
                warn!("[compression] Could not write all file {path} to zip: {err:?}");
                continue;
            }
        }
    }
    let finish_result = zip_writer.finish();
    match finish_result {
        Ok(_) => {}
        Err(err) => {
            warn!("[compression] Could not finish compressing to zip: {err:?}");
        }
    }
    Ok(())
}

#[cfg(target_os = "windows")]
/// Decompress seven bit compression
pub(crate) fn decompress_seven_bit(data: &[u8]) -> Result<Vec<u8>, ArtemisError> {
    let mut decompressed_data: Vec<u8> = Vec::new();
    let mut index: u16 = 0;
    let mut bit_value = 0;

    let max_value = 7;
    let bit_op = 0x7f;

    for value in data {
        bit_value |= (value.to_owned() as u16) << index;
        decompressed_data.push((bit_value & bit_op) as u8);
        bit_value >>= max_value;

        index += 1;

        if index == max_value {
            decompressed_data.push((bit_value & bit_op) as u8);
            bit_value >>= max_value;
            index = 0;
        }
    }
    Ok(decompressed_data)
}

#[cfg(target_os = "windows")]
/// Decompress LZXPRESS HUFFMAN. Must specify format. Ex: 4 = Huffman, 3 = xpress, 2 = lznt1, 1 = default, 0 = none
pub(crate) fn decompress_lzxpress_huffman(
    data: &mut [u8],
    decompress_size: u32,
    format: u16,
) -> Result<Vec<u8>, ArtemisError> {
    use ntapi::{
        ntrtl::{RtlDecompressBufferEx, RtlGetCompressionWorkSpaceSize},
        winapi::um::winnt::PVOID,
    };

    let mut buffer_workspace_size: u32 = 0;
    let mut frag_workspace_size: u32 = 0;
    let mut decom_size = 0;

    let decomcompress_result = decompress_size.try_into();
    let decomcompress_usize: usize = match decomcompress_result {
        Ok(result) => result,
        Err(err) => {
            error!("[compression] Failed to set uncompressed data size: {err:?}");
            return Err(ArtemisError::HuffmanCompression);
        }
    };

    let mut decompress_data: Vec<u8> = Vec::with_capacity(decomcompress_usize);
    let success = 0;

    // Make two calls to Windows APIs to decompress the data
    #[allow(unsafe_code)]
    unsafe {
        let status = RtlGetCompressionWorkSpaceSize(
            format,
            &mut buffer_workspace_size,
            &mut frag_workspace_size,
        );
        if status != success {
            error!("[compression] Failed to get lzxpress huffmane workspace size: {status}");
            return Err(ArtemisError::HuffmanCompression);
        }

        let frag_result = frag_workspace_size.try_into();
        let frag_size: usize = match frag_result {
            Ok(result) => result,
            Err(err) => {
                error!("[compression] Failed to get fragment workspace size data: {err:?}");
                return Err(ArtemisError::HuffmanCompression);
            }
        };
        let mut frag_data_size: Vec<PVOID> = Vec::with_capacity(frag_size);

        let status = RtlDecompressBufferEx(
            format,
            decompress_data.as_mut_ptr(),
            decompress_size,
            data.as_mut_ptr(),
            data.len() as u32,
            &mut decom_size,
            frag_data_size.as_mut_ptr().cast::<std::ffi::c_void>(),
        );
        if status != success {
            error!("[compression] Failed to decompress data: {status}");
            return Err(ArtemisError::HuffmanCompression);
        }
        decompress_data.set_len(decom_size as usize);

        Ok(decompress_data)
    }
}

#[cfg(test)]
mod tests {
    use super::compress_gzip_data;
    use crate::{
        filesystem::files::read_file,
        utils::compression::{compress_gzip, compress_output_zip},
    };
    use std::{fs::remove_file, path::PathBuf};

    #[test]
    #[cfg(target_os = "macos")]
    fn test_decompress_gzip() {
        use crate::utils::compression::decompress_gzip;

        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/macos/fsevents/DLS2/0000000000027d79");
        let files = decompress_gzip(&test_location.display().to_string()).unwrap();
        assert_eq!(files.len(), 78970);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_decompress_lzxpress_huffman() {
        use crate::utils::compression::decompress_lzxpress_huffman;
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/windows/compression/lz_huffman.raw");
        let mut bytes = read_file(&test_location.display().to_string()).unwrap();
        let huffman = 4;
        let files = decompress_lzxpress_huffman(&mut bytes, 153064, huffman).unwrap();
        assert_eq!(files.len(), 153064);
    }

    #[test]
    fn test_compress_file() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files/compressme.txt");
        let _ = compress_gzip(&test_location.display().to_string()).unwrap();
        test_location.pop();
        test_location.push("compressme.txt.gz");

        let data = read_file(&test_location.display().to_string()).unwrap();
        assert_eq!(data.len(), 89);
        remove_file(&test_location.display().to_string()).unwrap();
    }
    #[test]
    #[cfg(target_os = "linux")]
    fn test_decompress_zstd() {
        use super::decompress_zstd;

        let test_data = [
            40, 181, 47, 253, 96, 246, 1, 13, 11, 0, 38, 86, 70, 34, 48, 79, 220, 104, 104, 164,
            213, 236, 199, 164, 19, 243, 36, 222, 54, 232, 158, 27, 205, 124, 87, 133, 215, 237,
            160, 61, 33, 255, 131, 30, 20, 52, 81, 42, 62, 0, 59, 0, 61, 0, 11, 131, 33, 196, 25,
            210, 75, 130, 113, 33, 146, 97, 53, 111, 247, 103, 181, 12, 105, 217, 149, 183, 112,
            45, 173, 22, 87, 62, 248, 203, 155, 98, 89, 184, 157, 136, 113, 221, 146, 112, 201,
            134, 243, 130, 93, 228, 89, 237, 17, 241, 120, 60, 11, 106, 48, 120, 20, 14, 32, 232,
            121, 8, 213, 244, 202, 19, 135, 53, 1, 7, 160, 125, 197, 162, 176, 210, 24, 121, 122,
            101, 170, 229, 211, 231, 173, 170, 141, 157, 87, 253, 236, 14, 158, 59, 33, 136, 159,
            220, 201, 69, 73, 207, 165, 0, 193, 32, 17, 177, 0, 45, 170, 157, 2, 190, 92, 159, 75,
            98, 244, 244, 1, 77, 227, 73, 168, 84, 187, 37, 194, 165, 222, 18, 121, 17, 7, 41, 135,
            210, 83, 153, 126, 37, 125, 217, 240, 37, 207, 8, 43, 236, 59, 189, 198, 117, 29, 23,
            205, 152, 178, 154, 168, 38, 117, 20, 232, 3, 193, 59, 97, 194, 197, 72, 2, 88, 99,
            154, 129, 165, 160, 60, 184, 8, 73, 195, 26, 67, 208, 176, 2, 231, 185, 14, 203, 195,
            105, 42, 199, 247, 122, 70, 142, 0, 207, 101, 119, 81, 210, 96, 205, 97, 19, 82, 28,
            37, 0, 1, 173, 193, 176, 143, 148, 189, 157, 62, 199, 74, 106, 74, 191, 226, 189, 115,
            148, 228, 46, 68, 9, 99, 90, 19, 25, 0, 8, 48, 179, 128, 201, 15, 71, 22, 170, 254, 39,
            8, 216, 246, 107, 136, 75, 38, 214, 245, 184, 88, 200, 89, 197, 179, 101, 209, 103,
            196, 201, 9, 27, 133, 6, 11, 67, 204, 216, 132, 63, 226, 133, 45, 4, 177, 5, 85, 18,
            182, 230, 176, 178, 215, 245, 107, 134, 127, 83, 173, 195, 245, 106, 25, 9, 33, 10,
        ];
        let result = decompress_zstd(&test_data).unwrap();
        assert_eq!(result.len(), 758);
    }

    #[test]
    fn test_compress_gzip_data() {
        let data = "compressme".as_bytes();
        let results = compress_gzip_data(data).unwrap();
        assert_eq!(results.len(), 30)
    }

    #[test]
    fn test_compress_output_zip() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/system/files");
        let _ = compress_output_zip(&test_location.display().to_string(), "compressme").unwrap();
        test_location.push("compressme.zip");

        let data = read_file("compressme.zip").unwrap();
        assert!(data.len() < 500);
        remove_file("compressme.zip").unwrap();
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_decompress_seven_bit() {
        use super::decompress_seven_bit;

        let test = [213, 121, 89, 62, 7];
        let result = decompress_seven_bit(&test).unwrap();
        assert_eq!(result, [85, 115, 101, 114, 115]);
    }
}
