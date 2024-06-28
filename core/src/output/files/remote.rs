use super::error::AcquireError;
use crate::{
    artifacts::os::systeminfo::info::get_info_metadata,
    filesystem::{
        files::file_reader,
        metadata::{get_metadata, get_timestamps},
    },
    output::remote::{
        aws::{aws_complete_multipart, aws_creds, aws_multipart_upload, setup_upload},
        gcp::{create_jwt_gcp, gcp_get_upload_status, gcp_session, setup_gcp_upload},
    },
    structs::toml::Output,
};
use flate2::{write::GzEncoder, Compression};
use log::error;
use reqwest::{blocking::Client, StatusCode, Url};
use rusty_s3::{Bucket, Credentials};
use serde::Serialize;
use std::{collections::HashMap, fs::File};

pub(crate) struct AcquireFileApiRemote {
    pub(crate) path: String,
    pub(crate) filename: String,
    pub(crate) output: Output,
    pub(crate) md5: String,
    pub(crate) remote: RemoteType,
    pub(crate) session: String,
    pub(crate) token: String,
    pub(crate) bucket: Option<Bucket>,
    pub(crate) aws_creds: Option<Credentials>,
    pub(crate) aws_tags: Vec<String>,
    pub(crate) aws_id: u16,
}

#[derive(PartialEq)]
pub(crate) enum RemoteType {
    Gcp,
    Azure,
    Aws,
}

pub(crate) trait AcquireActionRemote {
    fn reader(&self) -> Result<File, AcquireError>;
    fn compressor(&self) -> GzEncoder<Vec<u8>>;
    fn upload_setup(&mut self) -> Result<(), AcquireError>;
    fn upload(
        &mut self,
        bytes: &[u8],
        offset: &usize,
        total_size: &str,
    ) -> Result<(), AcquireError>;
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

trait AmazonUpload {
    fn aws_start(&mut self) -> Result<(), AcquireError>;
    fn aws_upload(&mut self, bytes: &[u8]) -> Result<(), AcquireError>;
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
        match self.remote {
            RemoteType::Gcp => self.gcp_start()?,
            RemoteType::Azure => todo!(),
            RemoteType::Aws => self.aws_start()?,
        }

        Ok(())
    }

    /// Begin uploading data to cloud services
    fn upload(
        &mut self,
        bytes: &[u8],
        offset: &usize,
        total_size: &str,
    ) -> Result<(), AcquireError> {
        match self.remote {
            RemoteType::Gcp => self.gcp_upload(bytes, offset, total_size)?,
            RemoteType::Azure => todo!(),
            RemoteType::Aws => self.aws_upload(bytes)?,
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

impl AmazonUpload for AcquireFileApiRemote {
    /// Start the upload process to AWS
    fn aws_start(&mut self) -> Result<(), AcquireError> {
        let info_results = aws_creds(self.output.api_key.as_ref().unwrap_or(&String::new()));
        let info = match info_results {
            Ok(result) => result,
            Err(err) => {
                error!("[artemis-core] Could not parse AWS creds: {err:?}");
                return Err(AcquireError::AwsSetup);
            }
        };

        let url = Url::parse("https://s3.amazonaws.com").unwrap();

        self.filename = format!(
            "{}/{}/{}.{}",
            self.output.directory, self.output.name, self.filename, self.output.format
        );

        let mut headers = HashMap::new();

        headers.insert(String::from("x-amz-meta-fullpath"), self.path.clone());
        headers.insert(String::from("x-amz-meta-filename"), self.filename.clone());
        headers.insert(
            String::from("x-amz-meta-hostname"),
            get_info_metadata().hostname,
        );
        headers.insert(
            String::from("x-amz-meta-endpoint-id"),
            self.output.endpoint_id.clone(),
        );
        headers.insert(
            String::from("x-amz-meta-collection-id"),
            self.output.collection_id.to_string(),
        );

        let timestamps_result = get_timestamps(&self.path);
        let meta_result = get_metadata(&self.path);
        // If both values are ok, add metadata to final request
        if meta_result.is_ok() && timestamps_result.is_ok() {
            let timestamps = timestamps_result.unwrap();
            let metadata = meta_result.unwrap();

            headers.insert(String::from("x-amz-meta-created"), timestamps.created);
            headers.insert(String::from("x-amz-meta-modified"), timestamps.modified);
            headers.insert(String::from("x-amz-meta-accessed"), timestamps.accessed);
            headers.insert(String::from("x-amz-meta-changed"), timestamps.changed);
            headers.insert(String::from("x-amz-meta-size"), metadata.len().to_string());
        }

        let setup_results = setup_upload(info, url, &self.filename, &headers);
        let setup = match setup_results {
            Ok(result) => result,
            Err(err) => {
                error!("[artemis-core] Could not setup AWS upload: {err:?}");
                return Err(AcquireError::AwsSetup);
            }
        };

        self.session = setup.session.upload_id().to_string();
        self.bucket = Some(setup.bucket);
        self.aws_creds = Some(setup.creds);

        Ok(())
    }

    /// Upload bytes to AWS
    fn aws_upload(&mut self, bytes: &[u8]) -> Result<(), AcquireError> {
        if self.aws_creds.is_none() || self.bucket.is_none() {
            error!("[artemis-core] AWS bucket and/or creds not setup");
            return Err(AcquireError::AwsUpload);
        }
        let bucket = self.bucket.as_ref().unwrap();
        let creds = self.aws_creds.as_ref().unwrap();

        if bytes.is_empty() {
            let etags: Vec<&str> = self.aws_tags.iter().map(|tag| tag as &str).collect();

            let status =
                aws_complete_multipart(bucket, creds, &self.filename, &self.session, etags);
            if status.is_err() {
                error!("[artemis-core] Could not finish AWS upload");
                return Err(AcquireError::AwsUpload);
            }

            self.aws_tags = Vec::new();
            return Ok(());
        }

        let result = aws_multipart_upload(
            bytes,
            &self.session,
            bucket,
            creds,
            &self.filename,
            self.aws_id,
        );
        let mut tags = match result {
            Ok(tags) => tags,
            Err(_err) => return Err(AcquireError::AwsUpload),
        };

        self.aws_tags.append(&mut tags);

        Ok(())
    }
}
