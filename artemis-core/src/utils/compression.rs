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
