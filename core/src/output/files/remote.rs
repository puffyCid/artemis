use super::error::AcquireError;
use crate::{
    artifacts::os::systeminfo::info::get_info_metadata,
    filesystem::{
        files::file_reader,
        metadata::{get_metadata, get_timestamps},
    },
    output::remote::gcp::{create_jwt_gcp, gcp_get_upload_status, gcp_session, setup_gcp_upload},
    structs::toml::Output,
};
use flate2::{write::GzEncoder, Compression};
use log::error;
use reqwest::{blocking::Client, StatusCode};
use serde::Serialize;
use std::fs::File;

pub(crate) struct AcquireFileApiRemote {
    pub(crate) path: String,
    pub(crate) filename: String,
    pub(crate) output: Output,
    pub(crate) md5: String,
    pub(crate) remote: RemoteType,
    pub(crate) session: String,
    pub(crate) token: String,
}

#[derive(PartialEq)]
pub(crate) enum RemoteType {
    Gcp,
    Azure,
    Aws,
}

#[derive(Serialize)]
struct AcquireMetadata {
    created: i64,
    modified: i64,
    accessed: i64,
    changed: i64,
    size: u64,
    full_path: String,
    filename: String,
    md5: String,
}

pub(crate) trait AcquireActionRemote {
    fn reader(&self) -> Result<File, AcquireError>;
    fn compressor(&self) -> GzEncoder<Vec<u8>>;
    fn upload_setup(&mut self) -> Result<(), AcquireError>;
    fn upload(&self, bytes: &[u8], offset: &usize, total_size: &str) -> Result<(), AcquireError>;
}

trait GoogleUpload {
    fn gcp_start(&mut self) -> Result<(), AcquireError>;
    fn gcp_upload(
        &self,
        bytes: &[u8],
        offset: &usize,
        total_size: &str,
    ) -> Result<(), AcquireError>;
}

impl AcquireActionRemote for AcquireFileApiRemote {
    /// Create a reader for user to read acquired file
    fn reader(&self) -> Result<File, AcquireError> {
        let reader_result = file_reader(&self.path);
        let reader = match reader_result {
            Ok(result) => result,
            Err(err) => {
                error!(
                    "[artemis-core] Failed to open file reader for{}: {err:?}",
                    &self.path
                );
                return Err(AcquireError::Reader);
            }
        };

        Ok(reader)
    }

    /// Compress the acquired file and upload to cloud service
    fn compressor(&self) -> GzEncoder<Vec<u8>> {
        GzEncoder::new(Vec::new(), Compression::default())
    }

    /// Setup the upload process
    fn upload_setup(&mut self) -> Result<(), AcquireError> {
        if self.remote == RemoteType::Gcp {
            self.gcp_start()?;
        }

        Ok(())
    }

    /// Begin uploading data to cloud services
    fn upload(&self, bytes: &[u8], offset: &usize, total_size: &str) -> Result<(), AcquireError> {
        if self.remote == RemoteType::Gcp {
            self.gcp_upload(bytes, offset, total_size)?;
        }
        Ok(())
    }
}

impl GoogleUpload for AcquireFileApiRemote {
    /// Start uploading data to GCP
    fn gcp_start(&mut self) -> Result<(), AcquireError> {
        let setup_result = setup_gcp_upload(&self.output, &self.filename);
        let setup = match setup_result {
            Ok(result) => result,
            Err(err) => {
                error!("[artemis-core] Could not setup GCP upload: {err:?}");
                return Err(AcquireError::GcpSetup);
            }
        };

        let session_url = format!("{}/o?uploadType=resumable&name={}", setup.url, setup.output,);

        let token_result = create_jwt_gcp(&setup.api_key);
        let token = match token_result {
            Ok(result) => result,
            Err(err) => {
                error!("[artemis-core] Could not create GCP token: {err:?}");
                return Err(AcquireError::GcpToken);
            }
        };

        let session_result = gcp_session(&session_url, &token);
        let session = match session_result {
            Ok(result) => result,
            Err(err) => {
                error!("[artemis-core] Could not setup GCP session: {err:?}");
                return Err(AcquireError::GcpSession);
            }
        };

        self.token = token;
        self.session = session;

        Ok(())
    }

    /// Upload data to GCP
    fn gcp_upload(
        &self,
        bytes: &[u8],
        offset: &usize,
        total_size: &str,
    ) -> Result<(), AcquireError> {
        let max = 15;

        let mut max_attempts = 0;
        while max_attempts < max {
            let client = Client::new();
            let mut builder = client.put(&self.session);
            builder = builder.header("Content-Length", bytes.len());

            builder = builder.header(
                "Content-Range",
                format!("bytes {offset}-{}/{total_size}", (offset + bytes.len() - 1)),
            );

            // This is the final request
            if total_size != "*" {
                builder = builder.header("x-goog-meta-fullpath", self.path.clone());
                builder = builder.header("x-goog-meta-filename", self.filename.clone());
                builder = builder.header("x-goog-meta-md5", self.md5.clone());

                builder = builder.header("x-goog-meta-hostname", get_info_metadata().hostname);
                builder =
                    builder.header("x-goog-meta-endpoint-id", self.output.endpoint_id.clone());
                builder = builder.header("x-goog-meta-collection-id", self.output.collection_id);

                let timestamps_result = get_timestamps(&self.path);
                let meta_result = get_metadata(&self.path);
                // If both values are ok, add metadata to final request
                if meta_result.is_ok() && timestamps_result.is_ok() {
                    let timestamps = timestamps_result.unwrap();
                    let metadata = meta_result.unwrap();

                    builder = builder.header("x-goog-meta-created", timestamps.created);
                    builder = builder.header("x-goog-meta-modified", timestamps.modified);
                    builder = builder.header("x-goog-meta-accessed", timestamps.accessed);
                    builder = builder.header("x-goog-meta-changed", timestamps.changed);
                    builder = builder.header("x-goog-meta-size", metadata.len());
                }
            }

            let res_result = builder.body(bytes.to_vec()).send();
            let res = match res_result {
                Ok(result) => result,
                Err(err) => {
                    error!(
                        "[artemis-core] Could not upload to GCP storage: {err:?}. Attempting again"
                    );
                    max_attempts += 1;
                    continue;
                }
            };

            if res.status() != StatusCode::OK
                && res.status() != StatusCode::CREATED
                && res.status() != StatusCode::PERMANENT_REDIRECT
            {
                error!(
                "[artemis-core] Non-200 and non-308 response from GCP storage: {:?}. Attempting again",
                res.text()
            );
                max_attempts += 1;
                continue;
            }

            // Check to make sure GCP received our upload
            let status_result = gcp_get_upload_status(&self.session, "*");
            if status_result.is_err() {
                error!(
                    "[artemis-core] Could not check status of upload: {:?}",
                    status_result.unwrap_err()
                );
                return Err(AcquireError::GcpStatus);
            }

            return Ok(());
        }
        error!("[artemis-core] Max attempts reached for uploading to Google Cloud");
        Err(AcquireError::MaxAttempts)
    }
}
