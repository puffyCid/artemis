use super::error::CompressionError;
use crate::filesystem::files::read_file;
use flate2::{write::GzEncoder, Compression};
use log::{error, warn};
use std::{fs::File, io::Write};
use walkdir::WalkDir;
use zip::{write::SimpleFileOptions, ZipWriter};

/// Compress provided data with GZIP
pub(crate) fn compress_gzip_data(data: &[u8]) -> Result<Vec<u8>, CompressionError> {
    let mut gz = GzEncoder::new(Vec::new(), Compression::default());
    let status = gz.write_all(data);
    match status {
        Ok(_) => {}
        Err(err) => {
            error!("[compression] Could not compress data with gzip: {err:?}");
            return Err(CompressionError::CompressCreate);
        }
    }
    let finish_status = gz.finish();

    let data = match finish_status {
        Ok(results) => results,
        Err(err) => {
            error!("[compression] Could not finish gzip compressing data: {err:?}");
            return Err(CompressionError::GzipFinish);
        }
    };
    Ok(data)
}

/// Compress the output directory to a zip file
pub(crate) fn compress_output_zip(directory: &str, zip_name: &str) -> Result<(), CompressionError> {
    let output_files = WalkDir::new(directory);

    let zip_file_result = File::create(format!("{zip_name}.zip"));
    let zip_file = match zip_file_result {
        Ok(result) => result,
        Err(err) => {
            error!("[compression] Could not create compressed zip: {err:?}");
            return Err(CompressionError::CompressCreate);
        }
    };
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);
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

#[cfg(test)]
mod tests {
    use super::compress_gzip_data;
    use crate::{filesystem::files::read_file, utils::compression::compress::compress_output_zip};
    use std::{fs::remove_file, path::PathBuf};

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
}
