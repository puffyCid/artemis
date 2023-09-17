use super::error::ArtemisError;
use crate::filesystem::files::read_file;
use flate2::{write::GzEncoder, Compression};
use log::{error, warn};
use std::{fs::File, io::Write};
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
    if decoder.read_to_end(&mut data).is_err() {
        return Err(ArtemisError::ZstdDecompresss);
    }
    Ok(data)
}

#[cfg(target_os = "linux")]
/// Decompress lz4 data
pub(crate) fn decompress_lz4(data: &[u8], decom_size: usize) -> Result<Vec<u8>, ArtemisError> {
    use lz4_flex::decompress;

    let decompress_result = decompress(data, decom_size);
    let decomp_data = match decompress_result {
        Ok(result) => result,
        Err(err) => {
            error!("[compression] Could not decompress lz4 data: {err:?}");
            return Err(ArtemisError::Lz4Decompresss);
        }
    };
    Ok(decomp_data)
}

#[cfg(target_os = "linux")]
/// Decompress xz data
pub(crate) fn decompress_xz(data: &[u8]) -> Result<Vec<u8>, ArtemisError> {
    use std::io::Read;
    use xz2::read::XzDecoder;

    let mut decompress = XzDecoder::new(data);
    let mut data: Vec<u8> = Vec::new();
    if decompress.read_to_end(&mut data).is_err() {
        error!("[compression] Could not decompress xz data");
        return Err(ArtemisError::XzDecompress);
    }

    Ok(data)
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
    use crate::{filesystem::files::read_file, utils::compression::compress_output_zip};
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
    #[cfg(target_os = "linux")]
    fn test_decompress_xz() {
        use super::decompress_xz;

        let test_data = [
            253, 55, 122, 88, 90, 0, 0, 0, 255, 18, 217, 65, 3, 192, 169, 3, 131, 8, 33, 1, 16, 0,
            0, 0, 211, 65, 240, 142, 224, 4, 2, 1, 161, 93, 0, 33, 147, 198, 137, 174, 252, 60, 42,
            46, 195, 29, 60, 248, 205, 175, 108, 16, 165, 45, 1, 213, 149, 248, 139, 156, 158, 105,
            187, 7, 81, 7, 83, 19, 97, 130, 4, 93, 160, 13, 49, 168, 225, 0, 97, 23, 97, 47, 127,
            26, 196, 167, 250, 7, 218, 185, 125, 28, 130, 124, 136, 220, 80, 181, 67, 25, 170, 234,
            69, 55, 209, 98, 15, 30, 131, 65, 191, 37, 197, 171, 228, 54, 232, 107, 93, 65, 193, 7,
            93, 142, 17, 193, 87, 108, 95, 219, 94, 155, 59, 118, 250, 190, 124, 7, 139, 26, 54,
            234, 214, 11, 85, 16, 198, 239, 117, 142, 188, 138, 0, 73, 71, 183, 118, 151, 145, 109,
            205, 48, 222, 249, 66, 113, 102, 147, 74, 121, 83, 72, 151, 66, 82, 126, 140, 77, 7,
            55, 170, 29, 67, 13, 202, 177, 118, 58, 29, 95, 8, 60, 238, 211, 135, 249, 130, 242, 5,
            91, 158, 254, 132, 145, 185, 35, 185, 87, 103, 141, 60, 231, 31, 140, 237, 97, 34, 91,
            222, 151, 130, 141, 195, 73, 55, 239, 7, 211, 45, 185, 57, 218, 119, 157, 113, 171, 27,
            75, 237, 41, 204, 198, 27, 73, 237, 251, 121, 0, 248, 118, 213, 174, 65, 126, 134, 151,
            203, 201, 56, 136, 176, 237, 80, 239, 19, 126, 60, 239, 99, 247, 59, 50, 119, 182, 234,
            174, 38, 66, 39, 206, 254, 244, 243, 231, 85, 118, 120, 26, 7, 176, 127, 211, 212, 203,
            15, 6, 187, 220, 46, 62, 126, 176, 113, 230, 70, 218, 163, 63, 20, 100, 184, 29, 132,
            104, 74, 186, 38, 225, 167, 156, 24, 232, 223, 198, 44, 4, 229, 152, 250, 221, 76, 139,
            156, 38, 127, 156, 57, 163, 233, 94, 167, 219, 95, 8, 12, 108, 34, 159, 167, 26, 248,
            164, 93, 121, 247, 249, 48, 82, 211, 162, 215, 238, 53, 157, 126, 246, 211, 209, 67,
            150, 240, 196, 25, 204, 46, 52, 205, 249, 182, 88, 53, 66, 142, 237, 174, 164, 172, 24,
            89, 86, 86, 218, 2, 33, 68, 63, 182, 204, 190, 198, 95, 99, 30, 82, 162, 163, 100, 128,
            136, 46, 213, 27, 74, 170, 202, 197, 44, 150, 250, 16, 250, 101, 7, 60, 203, 161, 90,
            127, 210, 56, 140, 121, 205, 245, 233, 86, 154, 180, 167, 107, 82, 228, 188, 244, 121,
            19, 81, 100, 18, 90, 77, 46, 0, 0, 0, 0, 0, 1, 185, 3, 131, 8, 0, 0, 221, 164, 207, 72,
            168, 0, 10, 252, 2, 0, 0, 0, 0, 0, 89, 90,
        ];
        let result = decompress_xz(&test_data).unwrap();
        assert_eq!(result.len(), 1027);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_decompress_lz4() {
        use super::decompress_lz4;

        let test_data = [
            255, 157, 77, 69, 83, 83, 65, 71, 69, 61, 74, 83, 32, 69, 82, 82, 79, 82, 58, 32, 69,
            120, 99, 101, 112, 116, 105, 111, 110, 32, 105, 110, 32, 99, 97, 108, 108, 98, 97, 99,
            107, 32, 102, 111, 114, 32, 115, 105, 103, 110, 97, 108, 58, 32, 112, 111, 115, 105,
            116, 105, 111, 110, 45, 99, 104, 97, 110, 103, 101, 100, 58, 32, 84, 121, 112, 101, 69,
            114, 114, 111, 114, 58, 32, 116, 104, 105, 115, 46, 95, 114, 101, 99, 116, 32, 105,
            115, 32, 110, 117, 108, 108, 10, 103, 101, 116, 67, 117, 114, 114, 101, 110, 116, 82,
            101, 99, 116, 64, 114, 101, 115, 111, 117, 114, 99, 101, 58, 47, 47, 47, 111, 114, 103,
            47, 103, 110, 111, 109, 101, 47, 115, 104, 101, 108, 108, 47, 117, 105, 47, 107, 101,
            121, 98, 111, 97, 114, 100, 46, 106, 115, 58, 53, 54, 49, 58, 50, 50, 10, 119, 114, 97,
            112, 112, 101, 114, 58, 0, 4, 243, 23, 103, 106, 115, 47, 109, 111, 100, 117, 108, 101,
            115, 47, 95, 108, 101, 103, 97, 99, 121, 46, 106, 115, 58, 56, 50, 58, 50, 50, 10, 95,
            111, 110, 70, 111, 99, 117, 115, 80, 180, 0, 18, 67, 179, 0, 15, 75, 0, 4, 15, 133, 0,
            2, 111, 54, 52, 51, 58, 50, 48, 133, 0, 42, 79, 101, 109, 105, 116, 115, 0, 4, 8, 190,
            0, 2, 97, 1, 229, 115, 46, 106, 115, 58, 49, 50, 56, 58, 50, 55, 10, 95, 115, 60, 1,
            114, 87, 105, 110, 100, 111, 119, 47, 96, 1, 18, 99, 80, 1, 2, 20, 0, 4, 220, 0, 63,
            73, 100, 60, 101, 0, 4, 15, 216, 0, 2, 112, 53, 51, 53, 58, 50, 49, 10,
        ];
        let result = decompress_lz4(&test_data, 514).unwrap();
        assert_eq!(result.len(), 514);
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

        let data = read_file("compressme.zip").unwrap();
        assert!(!data.is_empty());
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
