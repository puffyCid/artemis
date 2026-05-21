use crate::{
    output2::{
        config::OutputConfig,
        error::{OutputError, OutputResult},
        report::CollectionReport,
        sink::{
            output_handle::{OutputHandle, OutputLocation},
            output_sink::{LogOutput, OutputSink},
        },
    },
    utils::{encoding::base64_decode_standard, time::time_now, uuid::generate_uuid},
};
use flate2::{Compression, write::GzEncoder};
use jsonwebtoken::{Algorithm, EncodingKey, Header, encode};
use reqwest::{StatusCode, blocking::Client};
use serde::{Deserialize, Serialize};
use std::{
    fs::{File, create_dir_all, read, remove_file},
    io::Write,
    path::PathBuf,
};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadResponse {
    time_created: String,
    name: String,
}
#[derive(Deserialize)]
struct GcpKey {
    private_key_id: String,
    private_key: String,
    client_email: String,
}
#[derive(Serialize)]
struct JwtToken {
    iss: String,
    sub: String,
    scope: String,
    iat: u64,
    exp: u64,
}
enum UploadStatus {
    Complete,
    ResumeFrom(usize),
}

pub(crate) struct GcpSink {
    url: String,
    url_path: String,
    log_file: PathBuf,
    credential: String,
    collection_id: u64,
    compress: bool,
}

impl GcpSink {
    pub(crate) fn new(config: &OutputConfig) -> OutputResult<Self> {
        let url = match &config.url {
            Some(result) => result,
            None => return Err(OutputError::Sink(String::from("no bucket provided"))),
        };

        let key = match &config.api_key {
            Some(result) => result,
            None => return Err(OutputError::Sink(String::from("no GCP API key provided"))),
        };

        let url_path = format!("{}/{}", config.directory.display(), config.name);

        let log_file = config.directory.join(&config.name).join(format!(
            "artemis_{}_{}.log",
            config.collection_id,
            generate_uuid()
        ));
        Ok(Self {
            url: url.clone(),
            url_path,
            credential: key.clone(),
            collection_id: config.collection_id,
            compress: config.compress,
            log_file,
        })
    }

    fn object_path(&self, filename: &str) -> String {
        format!("{}%2F{filename}", self.encode_path(&self.url_path))
    }

    fn output_path(&self, artifact_name: &str, extension: &str) -> String {
        let uuid = generate_uuid();
        let filename = if self.compress {
            format!("{artifact_name}_{uuid}.{extension}.gz")
        } else {
            format!("{artifact_name}_{uuid}.{extension}")
        };

        self.object_path(&filename)
    }

    fn encode_path(&self, path: &str) -> String {
        path.trim_matches('/').replace('/', "%2F")
    }

    fn remote_location(&self, url_path: &str) -> String {
        url_path.replace("%2F", "/")
    }

    fn log_filename(&self) -> String {
        format!("artemis_{}_{}.log", self.collection_id, generate_uuid())
    }

    fn upload_bytes(&self, object_name: &str, data: Vec<u8>, mime_type: &str) -> OutputResult<()> {
        let session = format!("{}/o?uploadType=resumable&name={object_name}", self.url);
        let token = self.create_jwt()?;
        let session_uri = self.create_upload_session(&session, &token)?;
        let client = Client::new();
        let result = client
            .put(&session_uri)
            .header("Content-Type", mime_type)
            .header("Content-Length", data.len())
            .body(data.clone())
            .send();

        match result {
            Ok(response)
                if response.status() == StatusCode::OK
                    || response.status() == StatusCode::CREATED =>
            {
                if let Ok(bytes) = response.bytes() {
                    if let Ok(status) = serde_json::from_slice::<UploadResponse>(&bytes) {
                        log::info!(
                            "[forensics] Uploaded GCP object {} at {}",
                            status.name,
                            status.time_created
                        );
                    }
                }
                return Ok(());
            }
            Ok(response) => {
                log::error!(
                    "[forensics] Non-success response from GCP upload: {:?}",
                    response.text()
                );
                self.resume_upload(&session_uri, &data)?
            }
            Err(err) => {
                log::error!("[output2] Failed to upload to GCP: {err:?}");
                self.resume_upload(&session_uri, &data)?
            }
        }

        Ok(())
    }

    fn create_upload_session(&self, url: &str, token: &str) -> OutputResult<String> {
        let response = Client::new()
            .post(url)
            .bearer_auth(token)
            .header("Content-Length", 0)
            .send()
            .map_err(|err| OutputError::Sink(format!("failed to create GCP session: {err:?}")))?;

        if response.status() != StatusCode::OK {
            return Err(OutputError::Sink(format!(
                "non-success response from GCP session: {:?}",
                response.text()
            )));
        }

        let location = response.headers().get("Location").ok_or_else(|| {
            OutputError::Sink(String::from("missing GCP session Location header"))
        })?;
        location
            .to_str()
            .map(|value| value.to_string())
            .map_err(|err| OutputError::Sink(format!("invalid GCP session Location header: {err}")))
    }

    fn create_jwt(&self) -> OutputResult<String> {
        let decoded_key = base64_decode_standard(&self.credential)
            .map_err(|err| OutputError::Sink(format!("failed to decode GCP key: {err:?}")))?;
        let gcp_key: GcpKey = serde_json::from_slice(&decoded_key)
            .map_err(|err| OutputError::Sink(format!("failed to parse GCP key JSON: {err:?}")))?;

        let mut header = Header::new(Algorithm::RS256);
        header.kid = Some(gcp_key.private_key_id);

        let expire = 3600;
        let start = time_now();
        let payload = JwtToken {
            iss: gcp_key.client_email.clone(),
            sub: gcp_key.client_email,
            scope: String::from("https://www.googleapis.com/auth/devstorage.write_only"),
            iat: start,
            exp: start + expire,
        };

        let encoding =
            EncodingKey::from_rsa_pem(gcp_key.private_key.as_bytes()).map_err(|err| {
                OutputError::Sink(format!("failed to create GCP encoding key: {err:?}"))
            })?;

        encode(&header, &payload, &encoding)
            .map_err(|err| OutputError::Sink(format!("failed to create GCP JWT: {err:?}")))
    }

    fn resume_upload(&self, session_uri: &str, data: &[u8]) -> OutputResult<()> {
        let max_attempts = 15;
        for _ in 0..max_attempts {
            match self.upload_status(session_uri, data.len())? {
                UploadStatus::Complete => return Ok(()),
                UploadStatus::ResumeFrom(offset) => {
                    if offset >= data.len() {
                        return Ok(());
                    }
                    let end = data.len() - 1;
                    let remaining = data[offset..].to_vec();
                    let response = Client::new()
                        .put(session_uri)
                        .header("Content-Length", remaining.len())
                        .header(
                            "Content-Range",
                            format!("bytes {offset}-{end}/{}", data.len()),
                        )
                        .body(remaining)
                        .send();
                    match response {
                        Ok(response)
                            if response.status() == StatusCode::OK
                                || response.status() == StatusCode::CREATED =>
                        {
                            return Ok(());
                        }
                        Ok(response) => {
                            log::warn!(
                                "[forensics] GCP resume upload got response: {:?}",
                                response.text()
                            );
                        }
                        Err(err) => {
                            log::warn!("[forensics] GCP resume upload failed: {err:?}");
                        }
                    }
                }
            }
        }
        Err(OutputError::Sink(String::from(
            "max attempts reached for GCP upload",
        )))
    }

    fn upload_status(&self, session_uri: &str, upload_size: usize) -> OutputResult<UploadStatus> {
        let response = Client::new()
            .put(session_uri)
            .header("Content-Length", 0)
            .header("Content-Range", format!("bytes */{upload_size}"))
            .send()
            .map_err(|err| {
                OutputError::Sink(format!("failed to get GCP upload status: {err:?}"))
            })?;

        if response.status() == StatusCode::OK || response.status() == StatusCode::CREATED {
            return Ok(UploadStatus::Complete);
        }
        if response.status() != StatusCode::PERMANENT_REDIRECT {
            return Err(OutputError::Sink(format!(
                "unexpected GCP upload status response: {:?}",
                response.text()
            )));
        }
        let Some(range) = response.headers().get("Range") else {
            return Ok(UploadStatus::ResumeFrom(0));
        };

        let range = range
            .to_str()
            .map_err(|err| OutputError::Sink(format!("invalid GCP Range header: {err}")))?;
        let last_uploaded = range
            .trim_start_matches("bytes=")
            .split('-')
            .next_back()
            .ok_or_else(|| OutputError::Sink(format!("invalid GCP Range header: {range}")))?
            .parse::<usize>()
            .map_err(|err| OutputError::Sink(format!("invalid GCP Range value: {err}")))?;

        Ok(UploadStatus::ResumeFrom(last_uploaded + 1))
    }
}

impl OutputSink for GcpSink {
    fn write_artifact(
        &mut self,
        artifact_name: &str,
        extension: &str,
        mime_type: &str,
        encode: &mut dyn FnMut(&mut dyn Write) -> OutputResult<usize>,
    ) -> OutputResult<OutputHandle> {
        let upload_filename = self.output_path(artifact_name, extension);
        let mut data = Vec::new();

        let record_count = if self.compress {
            let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
            let count = encode(&mut gzip)?;
            data = gzip.finish()?;
            count
        } else {
            let count = encode(&mut data)?;
            count
        };

        self.upload_bytes(&upload_filename, data, mime_type)?;

        Ok(OutputHandle::artifact(
            artifact_name,
            OutputLocation::Remote(self.remote_location(&upload_filename)),
            record_count,
            extension,
            self.compress,
        ))
    }

    fn write_report(&mut self, report: &CollectionReport) -> OutputResult<OutputHandle> {
        let filename = format!("report_{}.json", generate_uuid());
        let upload_report = self.object_path(&filename);
        let data = serde_json::to_vec(report)?;

        self.upload_bytes(&upload_report, data, "application/json")?;
        Ok(OutputHandle::report(OutputLocation::Remote(
            self.remote_location(&upload_report),
        )))
    }

    fn create_log_file(&mut self) -> OutputResult<LogOutput> {
        create_dir_all(&self.log_file).map_err(|err| OutputError::io_path(&self.log_file, err))?;
        let log_name = self.log_filename();
        let path = self.log_file.join(log_name);
        let file = File::create(&path).map_err(|err| OutputError::io_path(&path, err))?;
        self.log_file = path.clone();
        Ok(LogOutput { path, file })
    }

    fn finalize(&mut self) -> OutputResult<()> {
        let filename = self
            .log_file
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| OutputError::Finalize(String::from("log file path has no filename")))?;
        let object_log = self.encode_path(filename);

        let data = read(&self.log_file).map_err(|err| OutputError::io_path(&self.log_file, err))?;
        self.upload_bytes(&object_log, data, "text/plain")?;
        let _ = remove_file(&self.log_file);

        Ok(())
    }
}
