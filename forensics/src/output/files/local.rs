use super::error::AcquireError;
use crate::{
    filesystem::{
        files::file_reader,
        metadata::{get_metadata, get_timestamps},
    },
    output::local::output::local_output,
    structs::toml::Output,
    utils::{compression::compress::compress_output_zip, uuid::generate_uuid},
};
use flate2::{Compression, write::GzEncoder};
use log::error;
use serde::Serialize;
use std::fs::{File, OpenOptions, create_dir_all, remove_dir, remove_file};

pub(crate) struct AcquireFileApi {
    pub(crate) path: String,
    pub(crate) filename: String,
    pub(crate) output: Output,
    pub(crate) md5: String,
}

#[derive(Serialize)]
struct AcquireMetadata {
    created: String,
    modified: String,
    accessed: String,
    changed: String,
    size: u64,
    full_path: String,
    filename: String,
    md5: String,
}

pub(crate) trait AcquireActionLocal {
    fn reader(&self) -> Result<File, AcquireError>;
    fn compressor(&self) -> Result<GzEncoder<File>, AcquireError>;
    fn finish(&self) -> Result<(), AcquireError>;
}

impl AcquireActionLocal for AcquireFileApi {
    /// Create a reader for user to read acquired file
    fn reader(&self) -> Result<File, AcquireError> {
        let reader_result = file_reader(&self.path);
        let reader = match reader_result {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "[forensics] Failed to open file reader for{}: {err:?}",
                    &self.path
                );
                return Err(AcquireError::Reader);
            }
        };

        Ok(reader)
    }

    /// Compress the acquired file
    fn compressor(&self) -> Result<GzEncoder<File>, AcquireError> {
        let output_path = format!("{}/{}", &self.output.directory, &self.output.name);

        let result = create_dir_all(&output_path);
        if result.is_err() {
            error!(
                "[forensics] Failed to create output directory for {output_path}. Error: {:?}",
                result.unwrap_err()
            );
            return Err(AcquireError::CreateDirectory);
        }

        let writer_result = OpenOptions::new()
            .write(true)
            .create(true)
            .open(format!("{output_path}/{}.gz", &self.filename));

        let writer = match writer_result {
            Ok(results) => results,
            Err(err) => {
                error!(
                    "[forensics] Failed to create output file {} at {output_path}. Error: {err:?}",
                    &self.filename
                );
                return Err(AcquireError::Compressor);
            }
        };
        Ok(GzEncoder::new(writer, Compression::default()))
    }

    /// Finish the file acquision by grabbing file metadata and compressing everything
    fn finish(&self) -> Result<(), AcquireError> {
        let timestamps_result = get_timestamps(&self.path);
        let timestamps = match timestamps_result {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Failed to get timestamps: {err:?}");
                return Err(AcquireError::Timestamps);
            }
        };

        let meta_result = get_metadata(&self.path);
        let meta = match meta_result {
            Ok(result) => result,
            Err(err) => {
                error!("[forensics] Failed to get metadata: {err:?}");
                return Err(AcquireError::Metadata);
            }
        };

        let acq_meta = AcquireMetadata {
            created: timestamps.created,
            modified: timestamps.modified,
            accessed: timestamps.accessed,
            changed: timestamps.changed,
            size: meta.len(),
            full_path: self.path.clone(),
            filename: self.filename.clone(),
            md5: self.md5.clone(),
        };

        let meta_bytes = serde_json::to_vec(&acq_meta).unwrap_or_default();
        let result = local_output(
            &meta_bytes,
            &self.output,
            &format!("{}-metadata", &self.filename),
            "json",
        );

        if result.is_err() {
            error!(
                "[forensics] Failed to serialize metadata: {:?}",
                result.unwrap_err()
            );
            return Err(AcquireError::Metadata);
        }

        let directory = format!("{}/{}", &self.output.directory, &self.output.name);
        let zip_name = format!("{}/{}", &self.output.directory, &generate_uuid());

        let zip_out = compress_output_zip(&directory, &zip_name);
        if zip_out.is_err() {
            error!(
                "[forensics] Failed to complete acquisition: {:?}",
                zip_out.unwrap_err()
            );
            return Err(AcquireError::ZipOutput);
        }

        let acq_file = format!("{directory}/{}.gz", &self.filename);
        let status = remove_file(acq_file);
        if status.is_err() {
            error!(
                "[forensics] Failed to remove acquired file: {:?}",
                status.unwrap_err()
            );
            return Err(AcquireError::Cleanup);
        }

        let acq_file_json = format!("{directory}/{}-metadata.json", &self.filename);
        let status = remove_file(acq_file_json);
        if status.is_err() {
            error!(
                "[forensics] Failed to remove acquired file metadata: {:?}",
                status.unwrap_err()
            );
            return Err(AcquireError::Cleanup);
        }

        let status = remove_dir(directory);
        if status.is_err() {
            error!(
                "[forensics] Failed to remove output directory name: {:?}",
                status.unwrap_err()
            );
            return Err(AcquireError::Cleanup);
        }

        Ok(())
    }
}
