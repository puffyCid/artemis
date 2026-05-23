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
    utils::{encoding::base64_decode_standard, uuid::generate_uuid},
};
use flate2::{Compression, write::GzEncoder};
use log::{error, warn};
use reqwest::{StatusCode, blocking::Client, header::ETAG};
use rusty_s3::{
    Bucket, Credentials, S3Action, UrlStyle,
    actions::{
        CompleteMultipartUpload, CreateMultipartUpload, CreateMultipartUploadResponse, UploadPart,
    },
};
use serde::Deserialize;
use std::{
    fs::{File, read, remove_file},
    io::Write,
    path::PathBuf,
    time::Duration,
};
use url::Url;

#[derive(Deserialize)]
struct AwsInfo {
    bucket: String,
    secret: String,
    key: String,
    region: String,
}

struct AwsSetup {
    bucket: Bucket,
    creds: Credentials,
    session: CreateMultipartUploadResponse,
}

/// A data Sink representing the AWS pipeline flow
pub(crate) struct AwsSink {
    /// Full URL to AWS Bucket
    url: Url,
    /// Full URL that we upload our data too. Contains directory and name of out collection
    url_path: String,
    /// Local log file we are using to log any issues during the Artemis execution
    log_file: PathBuf,
    /// JSON credential used for uploads
    credential: String,
    /// Collection ID for the Artemis execution
    collection_id: u64,
    /// Whether to compress the results with gzip
    compress: bool,
}

impl AwsSink {
    /// Create a AWS sink and construct the URL target for the uploads
    pub(crate) fn new(config: &OutputConfig) -> OutputResult<Self> {
        let url = match &config.url {
            Some(result) => result,
            None => return Err(OutputError::Sink(String::from("no AWS bucket provided"))),
        };

        let key = match &config.api_key {
            Some(result) => result,
            None => return Err(OutputError::Sink(String::from("no AWS API key provided"))),
        };

        // Full URL path that we upload data to. Our directory and collection name will be folders in AWS
        // This mimics what Artemis does when writing to local disk
        let url_path = format!("{}/{}", config.directory.display(), config.name);

        // Local directory to store log file. Artemis logs issues locally. Once artifact and report uploads are done
        // The log file is then uploaded. The log file is uploaded last
        let log_file = config.directory.join(&config.name);
        let url = url
            .parse::<Url>()
            .map_err(|err| OutputError::Sink(format!("failed to parse AWS URL: {err:?}")))?;
        Ok(Self {
            url,
            url_path,
            credential: key.clone(),
            collection_id: config.collection_id,
            compress: config.compress,
            log_file,
        })
    }

    /// Start the upload process to AWS
    fn upload_bytes(&self, object_name: &str, data: Vec<u8>, mime_type: &str) -> OutputResult<()> {
        let info = self.decode_creds()?;
        let session = self.create_upload_session(info, object_name)?;
        // Valid for one (1) hour
        let duration = Duration::from_secs(3600);

        let part_upload = UploadPart::new(
            &session.bucket,
            Some(&session.creds),
            object_name,
            1,
            session.session.upload_id(),
        );

        let max_attempts = 15;
        let client = Client::new();
        let mut etag = Vec::new();
        for _ in 0..max_attempts {
            let signed_url = part_upload.sign(duration);
            let result = client
                .put(signed_url)
                .header("Content-Type", mime_type)
                .header("Content-Length", data.len())
                .body(data.clone())
                .send();
            match result {
                Ok(response) if response.status() == StatusCode::OK => {
                    if let Some(etag_header) = response.headers().get(ETAG) {
                        etag.push(etag_header.to_str().unwrap_or_default().to_string());
                        break;
                    }
                }
                Ok(response) => {
                    log::error!(
                        "[forensics] Non-OK response from AWS upload: {:?}",
                        response.text()
                    );
                }
                Err(err) => {
                    log::error!("[forensics] Failed to upload to AWS: {err:?}");
                }
            }
        }
        if etag.is_empty() {
            return Err(OutputError::Sink(String::from(
                "Could not start AWS upload. Zero etags",
            )));
        }
        let etags: Vec<&str> = etag.iter().map(|tag| tag as &str).collect();

        let action = CompleteMultipartUpload::new(
            &session.bucket,
            Some(&session.creds),
            object_name,
            session.session.upload_id(),
            etags.into_iter(),
        );
        let url = action.sign(duration);

        for _ in 0..max_attempts {
            let result = client.post(url.as_str()).body(action.clone().body()).send();
            match result {
                Ok(response) if response.status() == StatusCode::OK => {
                    if response
                        .text()
                        .unwrap_or_default()
                        .contains("Internal Error")
                    {
                        error!(
                            "[forensics] OK response on final upload but the response contained an error"
                        );
                        continue;
                    }
                    break;
                }
                Ok(response) => {
                    warn!("[forensics] Non-OK response on final upload Response: {response:?}");
                }
                Err(err) => {
                    log::error!("[forensics] Final upload failed to upload to AWS: {err:?}");
                }
            }
        }
        Ok(())
    }

    /// Construct the full path for our upload
    fn object_path(&self, filename: &str) -> String {
        format!("{}/{filename}", self.url_path)
    }

    /// Construct the filename for our upload
    fn output_path(&self, artifact_name: &str, extension: &str) -> String {
        let uuid = generate_uuid();
        let filename = if self.compress {
            format!("{artifact_name}_{uuid}.{extension}.gz")
        } else {
            format!("{artifact_name}_{uuid}.{extension}")
        };

        self.object_path(&filename)
    }

    /// Return the log file we are logging to
    fn log_filename(&self) -> String {
        format!("artemis_{}_{}.log", self.collection_id, generate_uuid())
    }

    /// Deserialize the base64 blob to our `AwsInfo` structure
    fn decode_creds(&self) -> OutputResult<AwsInfo> {
        let decoded_key = base64_decode_standard(&self.credential)
            .map_err(|err| OutputError::Sink(format!("failed to decode AWS key: {err:?}")))?;
        let aws_key: AwsInfo = serde_json::from_slice(&decoded_key)
            .map_err(|err| OutputError::Sink(format!("failed to parse AWS key JSON: {err:?}")))?;

        Ok(aws_key)
    }

    /// Setup the AWS upload session using our credentials
    fn create_upload_session(&self, info: AwsInfo, object_name: &str) -> OutputResult<AwsSetup> {
        let bucket = Bucket::new(
            self.url.clone(),
            UrlStyle::VirtualHost,
            info.bucket,
            info.region,
        )
        .map_err(|err| OutputError::Sink(format!("failed to initialize AWS bucket: {err:?}")))?;

        let creds = Credentials::new(info.key, info.secret);
        // Valid for one hour
        let duration = Duration::from_secs(3600);
        let action = CreateMultipartUpload::new(&bucket, Some(&creds), object_name);
        let url = action.sign(duration);

        let client = Client::new();
        let max_attempts = 15;

        let mut xml_session = String::new();
        for _ in 0..max_attempts {
            let response = client
                .post(url.as_str())
                .send()
                .map_err(|err| OutputError::Sink(format!("failed to start AWS upload: {err:?}")))?;

            if response.status() != StatusCode::OK {
                warn!(
                    "[forensics] Non-200 AWS response on upload start. Response: {:?}",
                    response.status()
                );
                continue;
            }
            xml_session = response.text().map_err(|err| {
                OutputError::Sink(format!("failed to get AWS XML start: {err:?}"))
            })?;
            break;
        }

        let session = CreateMultipartUpload::parse_response(xml_session).map_err(|err| {
            OutputError::Sink(format!("failed to parse AWS XML session: {err:?}"))
        })?;

        let setup = AwsSetup {
            bucket,
            creds,
            session,
        };

        Ok(setup)
    }
}

impl OutputSink for AwsSink {
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
            encode(&mut data)?
        };

        self.upload_bytes(&upload_filename, data, mime_type)?;

        Ok(OutputHandle::artifact(
            artifact_name,
            OutputLocation::Remote(format!("{}/{upload_filename}", self.url.as_str())),
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
        let data = serde_json::to_vec(report)?;

        self.upload_bytes(&upload_report, data, "application/json")?;
        Ok(OutputHandle::report(OutputLocation::Remote(format!(
            "{}/{upload_report}",
            self.url.as_str()
        ))))
    }

    fn create_log_file(&mut self) -> OutputResult<LogOutput> {
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
        self.upload_bytes(&object_log, data, "text/plain")?;
        let _ = remove_file(&self.log_file);

        Ok(())
    }
}
