use crate::{
    output::{
        error::{OutputError, OutputResult},
        report::CollectionReport,
        sink::{
            output_handle::{OutputHandle, OutputLocation},
            output_sink::{LogOutput, OutputSink},
        },
    },
    structs::toml::OutputConfig,
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
use tracing::{error, info, warn};

/// GCP response upload successful upload
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UploadResponse {
    /// Timestamp file was created upon uplad
    time_created: String,
    /// Name of file
    name: String,
}

/// Key used to authenticate to GCP
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

/// Status we track when uploading files
#[derive(Debug, PartialEq)]
enum UploadStatus {
    /// Upload is done
    Complete,
    /// Need to resume large upload
    ResumeFrom(usize),
}

/// A data Sink representing the GCP pipeline flow
pub(crate) struct GcpSink {
    /// Full URL to GCP Bucket
    url: String,
    /// Full object that we upload our data too. Contains directory and name of out collection
    object_prefix: String,
    /// Local log file we are using to log any issues during the Artemis execution
    log_file: PathBuf,
    /// JSON credential used for uploads
    credential: String,
    /// Collection ID for the Artemis execution
    collection_id: u64,
    /// Whether to compress the results with gzip
    compress: bool,
}

impl GcpSink {
    /// Create a GCP sink and construct the URL target for the uploads
    pub(crate) fn new(config: &OutputConfig) -> OutputResult<Self> {
        let url = match &config.url {
            Some(result) => result,
            None => return Err(OutputError::Sink(String::from("no GCP bucket provided"))),
        };

        let key = match &config.api_key {
            Some(result) => result,
            None => return Err(OutputError::Sink(String::from("no GCP API key provided"))),
        };

        // Full path that we upload data to. Our directory and collection name will be folders in GCP
        // This mimics what Artemis does when writing to local disk
        let object_prefix = format!("{}/{}", config.directory.display(), config.name);

        // Local directory to store log file. Artemis logs issues locally. Once artifact and report uploads are done
        // The log file is then uploaded. The log file is uploaded last
        let log_file = config.directory.join(&config.name);
        Ok(Self {
            url: url.clone(),
            object_prefix,
            credential: key.clone(),
            collection_id: config.collection_id,
            compress: config.compress,
            log_file,
        })
    }

    /// Encode our uploaded filenames
    fn object_path(&self, filename: &str) -> String {
        format!("{}%2F{filename}", GcpSink::encode_path(&self.object_prefix))
    }

    /// Construct the full path for our upload
    fn construct_filename(&self, artifact_name: &str, extension: &str) -> String {
        let uuid = generate_uuid();
        let filename = if self.compress {
            format!("{artifact_name}_{uuid}.{extension}.gz")
        } else {
            format!("{artifact_name}_{uuid}.{extension}")
        };

        self.object_path(&filename)
    }

    /// URL encode upload paths to "%2F"
    fn encode_path(path: &str) -> String {
        path.trim_matches('/').replace('/', "%2F")
    }

    /// URL decode upload paths to "/"
    fn remote_location(object_prefix: &str) -> String {
        object_prefix.replace("%2F", "/")
    }

    /// Return the log file we are logging to
    fn log_filename(&self) -> String {
        format!("artemis_{}_{}.jsonl", self.collection_id, generate_uuid())
    }

    /// Start the upload process to GCP
    fn upload_bytes(&self, object_name: &str, data: Vec<u8>, mime_type: &str) -> OutputResult<()> {
        let session = format!("{}/o?uploadType=resumable&name={object_name}", self.url);
        let token = self.create_jwt()?;
        let session_uri = GcpSink::create_upload_session(&session, &token)?;
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
                if let Ok(bytes) = response.bytes()
                    && let Ok(status) = serde_json::from_slice::<UploadResponse>(&bytes)
                {
                    info!(
                        "Uploaded GCP object {} at {}",
                        status.name, status.time_created
                    );
                }
                return Ok(());
            }
            Ok(response) => {
                error!("Non-OK response from GCP upload: {:?}", response.text());
                // Retry the upload 15 times
                GcpSink::resume_upload(&session_uri, &data)?;
            }
            Err(err) => {
                error!("Failed to upload to GCP: {err:?}");
                // Retry the upload 15 times
                GcpSink::resume_upload(&session_uri, &data)?;
            }
        }

        Ok(())
    }

    /// Initialize the GCP upload session to start uploading data
    fn create_upload_session(url: &str, token: &str) -> OutputResult<String> {
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

    /// Generate JWT token based on provided JSON service object
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

    fn resume_upload(session_uri: &str, data: &[u8]) -> OutputResult<()> {
        let max_attempts = 15;
        for attempt in 0..max_attempts {
            match GcpSink::upload_status(session_uri, data.len())? {
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
                            warn!(
                                "GCP resume issue on attempt {attempt}: {:?}",
                                response.text()
                            );
                        }
                        Err(err) => {
                            warn!("GCP resume failed on attempt {attempt}: {err:?}");
                        }
                    }
                }
            }
        }
        Err(OutputError::Sink(String::from(
            "max attempts reached for GCP upload",
        )))
    }

    /// Check our upload status when resume uploads. We try to resume any interrupted uploads
    fn upload_status(session_uri: &str, upload_size: usize) -> OutputResult<UploadStatus> {
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
        let upload_filename = self.construct_filename(artifact_name, extension);
        let mut data = Vec::new();

        let record_count = if self.compress {
            let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
            let count = encode(&mut gzip)?;
            data = gzip.finish()?;
            count
        } else {
            encode(&mut data)?
        };

        self.upload_bytes(&upload_filename, data, mime_type)?;

        Ok(OutputHandle::artifact(
            OutputLocation::Remote(GcpSink::remote_location(&upload_filename)),
            record_count,
        ))
    }

    fn write_report(&mut self, report: &CollectionReport) -> OutputResult<OutputHandle> {
        let filename = format!("report_{}.json", generate_uuid());
        let upload_report = self.object_path(&filename);
        let data = serde_json::to_vec(report)?;

        self.upload_bytes(&upload_report, data, "application/json")?;
        Ok(OutputHandle::report(OutputLocation::Remote(
            GcpSink::remote_location(&upload_report),
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
        let object_log = self.object_path(filename);

        let data = read(&self.log_file).map_err(|err| OutputError::io_path(&self.log_file, err))?;
        self.upload_bytes(&object_log, data, "application/jsonl")?;
        let _ = remove_file(&self.log_file);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::output::error::OutputError;
    use crate::output::sink::gcp::{GcpSink, UploadStatus};
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use httpmock::Method::{POST, PUT};
    use httpmock::MockServer;
    use serde_json::json;
    use std::path::PathBuf;

    fn gcp_config(port: u16) -> OutputConfig {
        OutputConfig {
            name: String::from("test"),
            endpoint_id: String::from("test"),
            collection_id: 0,
            directory: PathBuf::from("./tmp"),
            destination: OutputDestination::Gcp,
            format: OutputFormat::Csv,
            url: Some(format!("http://127.0.0.1:{port}")),
            api_key: Some(String::from(
                "ewogICJ0eXBlIjogInNlcnZpY2VfYWNjb3VudCIsCiAgInByb2plY3RfaWQiOiAiZmFrZW1lIiwKICAicHJpdmF0ZV9rZXlfaWQiOiAiZmFrZW1lIiwKICAicHJpdmF0ZV9rZXkiOiAiLS0tLS1CRUdJTiBQUklWQVRFIEtFWS0tLS0tXG5NSUlFdndJQkFEQU5CZ2txaGtpRzl3MEJBUUVGQUFTQ0JLa3dnZ1NsQWdFQUFvSUJBUUM3VkpUVXQ5VXM4Y0tqTXpFZll5amlXQTRSNC9NMmJTMUdCNHQ3TlhwOThDM1NDNmRWTXZEdWljdEdldXJUOGpOYnZKWkh0Q1N1WUV2dU5Nb1NmbTc2b3FGdkFwOEd5MGl6NXN4alptU25YeUNkUEVvdkdoTGEwVnpNYVE4cytDTE95UzU2WXlDRkdlSlpxZ3R6SjZHUjNlcW9ZU1c5YjlVTXZrQnBaT0RTY3RXU05HajNQN2pSRkRPNVZvVHdDUUFXYkZuT2pEZkg1VWxncDJQS1NRblNKUDNBSkxRTkZOZTdicjFYYnJoVi8vZU8rdDUxbUlwR1NEQ1V2M0UwRERGY1dEVEg5Y1hEVFRsUlpWRWlSMkJ3cFpPT2tFL1owL0JWbmhaWUw3MW9aVjM0YktmV2pRSXQ2Vi9pc1NNYWhkc0FBU0FDcDRaVEd0d2lWdU5kOXR5YkFnTUJBQUVDZ2dFQkFLVG1qYVM2dGtLOEJsUFhDbFRRMnZwei9ONnV4RGVTMzVtWHBxYXNxc2tWbGFBaWRnZy9zV3FwalhEYlhyOTNvdElNTGxXc00rWDBDcU1EZ1NYS2VqTFMyang0R0RqSTFaVFhnKyswQU1KOHNKNzRwV3pWRE9mbUNFUS83d1hzMytjYm5YaEtyaU84WjAzNnE5MlFjMStOODdTSTM4bmtHYTBBQkg5Q044M0htUXF0NGZCN1VkSHp1SVJlL21lMlBHaElxNVpCemo2aDNCcG9QR3pFUCt4M2w5WW1LOHQvMWNOMHBxSStkUXdZZGdmR2phY2tMdS8ycUg4ME1DRjdJeVFhc2VaVU9KeUtyQ0x0U0QvSWl4di9oekRFVVBmT0NqRkRnVHB6ZjNjd3RhOCtvRTR3SENvMWlJMS80VGxQa3dtWHg0cVNYdG13NGFRUHo3SURRdkVDZ1lFQThLTlRoQ08yZ3NDMkk5UFFETS84Q3cwTzk4M1dDRFkrb2krN0pQaU5BSnd2NURZQnFFWkIxUVlkajA2WUQxNlhsQy9IQVpNc01rdTFuYTJUTjBkcml3ZW5RUVd6b2V2M2cyUzdnUkRvUy9GQ0pTSTNqSitramd0YUE3UW16bGdrMVR4T0ROK0cxSDkxSFc3dDBsN1ZuTDI3SVd5WW8ycVJSSzNqenhxVWlQVUNnWUVBeDBvUXMycmVCUUdNVlpuQXBEMWplcTduNE12TkxjUHZ0OGIvZVU5aVV2Nlk0TWowU3VvL0FVOGxZWlhtOHViYnFBbHd6MlZTVnVuRDJ0T3BsSHlNVXJ0Q3RPYkFmVkRVQWhDbmRLYUE5Z0FwZ2ZiM3h3MUlLYnVRMXU0SUYxRkpsM1Z0dW1mUW4vL0xpSDFCM3JYaGNkeW8zL3ZJdHRFazQ4UmFrVUtDbFU4Q2dZRUF6VjdXM0NPT2xERGNRZDkzNURkdEtCRlJBUFJQQWxzcFFVbnpNaTVlU0hNRC9JU0xEWTVJaVFIYklIODNENGJ2WHEwWDdxUW9TQlNOUDdEdnYzSFl1cU1oZjBEYWVncmxCdUpsbEZWVnE5cVBWUm5LeHQxSWwySGd4T0J2YmhPVCs5aW4xQnpBK1lKOTlVekM4NU8wUXowNkErQ210SEV5NGFaMmtqNWhIakVDZ1lFQW1OUzQrQThGa3NzOEpzMVJpZUsyTG5pQnhNZ21ZbWwzcGZWTEtHbnptbmc3SDIrY3dQTGhQSXpJdXd5dFh5d2gyYnpic1lFZll4M0VvRVZnTUVwUGhvYXJRbllQdWtySk80Z3dFMm81VGU2VDVtSlNaR2xRSlFqOXE0WkIyRGZ6ZXQ2SU5zSzBvRzhYVkdYU3BRdlFoM1JVWWVrQ1pRa0JCRmNwcVdwYklFc0NnWUFuTTNEUWYzRkpvU25YYU1oclZCSW92aWM1bDB4RmtFSHNrQWpGVGV2Tzg2RnN6MUMyYVNlUktTcUdGb09RMHRtSnpCRXMxUjZLcW5ISW5pY0RUUXJLaEFyZ0xYWDR2M0NkZGpmVFJKa0ZXRGJFL0NrdktaTk9yY2YxbmhhR0NQc3BSSmoyS1VrajFGaGw5Q25jZG4vUnNZRU9OYndRU2pJZk1Qa3Z4Ris4SFE9PVxuLS0tLS1FTkQgUFJJVkFURSBLRVktLS0tLVxuIiwKICAiY2xpZW50X2VtYWlsIjogImZha2VAZ3NlcnZpY2VhY2NvdW50LmNvbSIsCiAgImNsaWVudF9pZCI6ICJmYWtlbWUiLAogICJhdXRoX3VyaSI6ICJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20vby9vYXV0aDIvYXV0aCIsCiAgInRva2VuX3VyaSI6ICJodHRwczovL29hdXRoMi5nb29nbGVhcGlzLmNvbS90b2tlbiIsCiAgImF1dGhfcHJvdmlkZXJfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9vYXV0aDIvdjEvY2VydHMiLAogICJjbGllbnRfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9yb2JvdC92MS9tZXRhZGF0YS94NTA5L2Zha2VtZSIsCiAgInVuaXZlcnNlX2RvbWFpbiI6ICJnb29nbGVhcGlzLmNvbSIKfQo=",
            )),
            ..Default::default()
        }
    }

    #[test]
    fn test_gcp_sink() {
        let server = MockServer::start();
        let port = server.port();
        let config = gcp_config(port);
        let sink = GcpSink::new(&config).unwrap();
        assert_eq!(
            sink.credential,
            "ewogICJ0eXBlIjogInNlcnZpY2VfYWNjb3VudCIsCiAgInByb2plY3RfaWQiOiAiZmFrZW1lIiwKICAicHJpdmF0ZV9rZXlfaWQiOiAiZmFrZW1lIiwKICAicHJpdmF0ZV9rZXkiOiAiLS0tLS1CRUdJTiBQUklWQVRFIEtFWS0tLS0tXG5NSUlFdndJQkFEQU5CZ2txaGtpRzl3MEJBUUVGQUFTQ0JLa3dnZ1NsQWdFQUFvSUJBUUM3VkpUVXQ5VXM4Y0tqTXpFZll5amlXQTRSNC9NMmJTMUdCNHQ3TlhwOThDM1NDNmRWTXZEdWljdEdldXJUOGpOYnZKWkh0Q1N1WUV2dU5Nb1NmbTc2b3FGdkFwOEd5MGl6NXN4alptU25YeUNkUEVvdkdoTGEwVnpNYVE4cytDTE95UzU2WXlDRkdlSlpxZ3R6SjZHUjNlcW9ZU1c5YjlVTXZrQnBaT0RTY3RXU05HajNQN2pSRkRPNVZvVHdDUUFXYkZuT2pEZkg1VWxncDJQS1NRblNKUDNBSkxRTkZOZTdicjFYYnJoVi8vZU8rdDUxbUlwR1NEQ1V2M0UwRERGY1dEVEg5Y1hEVFRsUlpWRWlSMkJ3cFpPT2tFL1owL0JWbmhaWUw3MW9aVjM0YktmV2pRSXQ2Vi9pc1NNYWhkc0FBU0FDcDRaVEd0d2lWdU5kOXR5YkFnTUJBQUVDZ2dFQkFLVG1qYVM2dGtLOEJsUFhDbFRRMnZwei9ONnV4RGVTMzVtWHBxYXNxc2tWbGFBaWRnZy9zV3FwalhEYlhyOTNvdElNTGxXc00rWDBDcU1EZ1NYS2VqTFMyang0R0RqSTFaVFhnKyswQU1KOHNKNzRwV3pWRE9mbUNFUS83d1hzMytjYm5YaEtyaU84WjAzNnE5MlFjMStOODdTSTM4bmtHYTBBQkg5Q044M0htUXF0NGZCN1VkSHp1SVJlL21lMlBHaElxNVpCemo2aDNCcG9QR3pFUCt4M2w5WW1LOHQvMWNOMHBxSStkUXdZZGdmR2phY2tMdS8ycUg4ME1DRjdJeVFhc2VaVU9KeUtyQ0x0U0QvSWl4di9oekRFVVBmT0NqRkRnVHB6ZjNjd3RhOCtvRTR3SENvMWlJMS80VGxQa3dtWHg0cVNYdG13NGFRUHo3SURRdkVDZ1lFQThLTlRoQ08yZ3NDMkk5UFFETS84Q3cwTzk4M1dDRFkrb2krN0pQaU5BSnd2NURZQnFFWkIxUVlkajA2WUQxNlhsQy9IQVpNc01rdTFuYTJUTjBkcml3ZW5RUVd6b2V2M2cyUzdnUkRvUy9GQ0pTSTNqSitramd0YUE3UW16bGdrMVR4T0ROK0cxSDkxSFc3dDBsN1ZuTDI3SVd5WW8ycVJSSzNqenhxVWlQVUNnWUVBeDBvUXMycmVCUUdNVlpuQXBEMWplcTduNE12TkxjUHZ0OGIvZVU5aVV2Nlk0TWowU3VvL0FVOGxZWlhtOHViYnFBbHd6MlZTVnVuRDJ0T3BsSHlNVXJ0Q3RPYkFmVkRVQWhDbmRLYUE5Z0FwZ2ZiM3h3MUlLYnVRMXU0SUYxRkpsM1Z0dW1mUW4vL0xpSDFCM3JYaGNkeW8zL3ZJdHRFazQ4UmFrVUtDbFU4Q2dZRUF6VjdXM0NPT2xERGNRZDkzNURkdEtCRlJBUFJQQWxzcFFVbnpNaTVlU0hNRC9JU0xEWTVJaVFIYklIODNENGJ2WHEwWDdxUW9TQlNOUDdEdnYzSFl1cU1oZjBEYWVncmxCdUpsbEZWVnE5cVBWUm5LeHQxSWwySGd4T0J2YmhPVCs5aW4xQnpBK1lKOTlVekM4NU8wUXowNkErQ210SEV5NGFaMmtqNWhIakVDZ1lFQW1OUzQrQThGa3NzOEpzMVJpZUsyTG5pQnhNZ21ZbWwzcGZWTEtHbnptbmc3SDIrY3dQTGhQSXpJdXd5dFh5d2gyYnpic1lFZll4M0VvRVZnTUVwUGhvYXJRbllQdWtySk80Z3dFMm81VGU2VDVtSlNaR2xRSlFqOXE0WkIyRGZ6ZXQ2SU5zSzBvRzhYVkdYU3BRdlFoM1JVWWVrQ1pRa0JCRmNwcVdwYklFc0NnWUFuTTNEUWYzRkpvU25YYU1oclZCSW92aWM1bDB4RmtFSHNrQWpGVGV2Tzg2RnN6MUMyYVNlUktTcUdGb09RMHRtSnpCRXMxUjZLcW5ISW5pY0RUUXJLaEFyZ0xYWDR2M0NkZGpmVFJKa0ZXRGJFL0NrdktaTk9yY2YxbmhhR0NQc3BSSmoyS1VrajFGaGw5Q25jZG4vUnNZRU9OYndRU2pJZk1Qa3Z4Ris4SFE9PVxuLS0tLS1FTkQgUFJJVkFURSBLRVktLS0tLVxuIiwKICAiY2xpZW50X2VtYWlsIjogImZha2VAZ3NlcnZpY2VhY2NvdW50LmNvbSIsCiAgImNsaWVudF9pZCI6ICJmYWtlbWUiLAogICJhdXRoX3VyaSI6ICJodHRwczovL2FjY291bnRzLmdvb2dsZS5jb20vby9vYXV0aDIvYXV0aCIsCiAgInRva2VuX3VyaSI6ICJodHRwczovL29hdXRoMi5nb29nbGVhcGlzLmNvbS90b2tlbiIsCiAgImF1dGhfcHJvdmlkZXJfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9vYXV0aDIvdjEvY2VydHMiLAogICJjbGllbnRfeDUwOV9jZXJ0X3VybCI6ICJodHRwczovL3d3dy5nb29nbGVhcGlzLmNvbS9yb2JvdC92MS9tZXRhZGF0YS94NTA5L2Zha2VtZSIsCiAgInVuaXZlcnNlX2RvbWFpbiI6ICJnb29nbGVhcGlzLmNvbSIKfQo="
        );

        assert!(!sink.log_file.display().to_string().is_empty());
        assert_eq!(sink.object_path("test"), ".%2Ftmp%2Ftest%2Ftest");
        assert!(
            sink.construct_filename("processes", "jsonl")
                .contains(".%2Ftmp%2Ftest%2Fprocesses_")
        );

        assert_eq!(GcpSink::encode_path("test/test/test"), "test%2Ftest%2Ftest");
        assert_eq!(
            GcpSink::remote_location("test%2Ftest%2Ftest"),
            "test/test/test"
        );
        assert!(sink.log_filename().contains("artemis_"));

        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200)
                .header("content-type", "application/json")
                .header("Location", format!("http://127.0.0.1:{port}"))
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });

        let mock_me_put = server.mock(|when, then| {
            when.method(PUT);
            then.status(200)
                .header("content-type", "application/json")
                .header("Location", format!("http://127.0.0.1:{port}"))
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });
        sink.upload_bytes(
            &sink.construct_filename("test", "jsonl"),
            vec![0, 0, 0, 0, 0],
            "application/jsonl",
        )
        .unwrap();
        mock_me.assert();
        mock_me_put.assert();
    }

    #[test]
    fn test_gcp_upload_session() {
        let server = MockServer::start();
        let port = server.port();
        let config = gcp_config(port);
        let sink = GcpSink::new(&config).unwrap();
        let object = &sink.construct_filename("test", "jsonl");
        let session = format!("{}/o?uploadType=resumable&name={object}", sink.url);
        let token = sink.create_jwt().unwrap();
        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200)
                .header("content-type", "application/json")
                .header("Location", format!("http://127.0.0.1:{port}"))
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });
        let session_uri = GcpSink::create_upload_session(&session, &token).unwrap();
        assert!(session_uri.contains("http://127.0.0.1:"));
        mock_me.assert();
    }

    #[test]
    fn test_gcp_create_jwt() {
        let config = gcp_config(0);
        let sink = GcpSink::new(&config).unwrap();

        assert!(sink.create_jwt().unwrap().len() > 40);
    }

    #[test]
    fn test_gcp_resume_upload() {
        let server = MockServer::start();
        let port = server.port();
        let config = gcp_config(port);
        let sink = GcpSink::new(&config).unwrap();

        let mock_me_post = server.mock(|when, then| {
            when.method(POST);
            then.status(200)
                .header("content-type", "application/json")
                .header("Location", format!("http://127.0.0.1:{port}"))
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });
        let mock_me_resume = server.mock(|when, then| {
            when.method(PUT)
                .header_exists("Content-Length")
                .header("Content-Range", "bytes 3-4/5");
            then.status(200)
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });
        let mock_me = server.mock(|when, then| {
            when.method(PUT)
                .header("Content-Range", "bytes */5")
                .header("Content-Length", "0");
            then.status(308)
                .header("Range", "0-2")
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });

        let data = [0, 1, 2, 3, 4];
        let object = &sink.construct_filename("test", "jsonl");
        let session = format!("{}/o?uploadType=resumable&name={object}", sink.url);
        let token = sink.create_jwt().unwrap();
        let session_uri = GcpSink::create_upload_session(&session, &token).unwrap();
        GcpSink::resume_upload(&session_uri, &data).unwrap();

        mock_me.assert();
        mock_me_resume.assert();
        mock_me_post.assert();
    }

    #[test]
    fn test_gcp_upload_status() {
        let server = MockServer::start();
        let port = server.port();

        let mock_me = server.mock(|when, then| {
            when.method(PUT);
            then.status(308)
                .header("Range", "0-5")
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });

        let bytes = GcpSink::upload_status(&format!("http://127.0.0.1:{port}"), 10).unwrap();

        assert_eq!(bytes, UploadStatus::ResumeFrom(6));
        mock_me.assert();
    }

    #[test]
    fn test_gcp_upload_status_complete() {
        let server = MockServer::start();
        let port = server.port();

        let mock_me = server.mock(|when, then| {
            when.method(PUT);
            then.status(200)
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });

        let bytes = GcpSink::upload_status(&format!("http://127.0.0.1:{port}"), 10).unwrap();

        assert_eq!(bytes, UploadStatus::Complete);
        mock_me.assert();
    }

    #[test]
    fn test_gcp_max_attempts() {
        let server = MockServer::start();
        let port = server.port();
        let mock_me = server.mock(|when, then| {
            when.method(PUT);
            then.status(308)
                .header("Range", "0-2")
                .json_body(json!({ "timeCreated": "whatever", "name":"mockme" }));
        });
        let data = [0, 1, 2, 3, 4];
        let err = GcpSink::resume_upload(&format!("http://127.0.0.1:{port}"), &data).unwrap_err();

        assert!(
            matches!(err, OutputError::Sink(value) if value == "max attempts reached for GCP upload")
        );
        mock_me.assert_calls(30);
    }

    #[test]
    fn test_gcp_bad_url() {
        let config = gcp_config(12345);
        let sink = GcpSink::new(&config).unwrap();
        let err = sink
            .upload_bytes("test", vec![0, 0, 0], "test")
            .unwrap_err();
        assert!(
            matches!(err, OutputError::Sink(value) if value.contains("failed to create GCP session: reqwest::Error "))
        )
    }
}
