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
    fs::{File, create_dir_all, read, remove_file},
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
    url_style: UrlStyle,
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

        // Path that we upload data to. Our directory and collection name will be folders in AWS
        // This mimics what Artemis does when writing to local disk
        let object_prefix = format!("{}/{}", config.directory.display(), config.name);

        // Local directory to store log file. Artemis logs issues locally. Once artifact and report uploads are done
        // The log file is then uploaded. The log file is uploaded last
        let log_file = config.directory.join(&config.name);
        let url = url
            .parse::<Url>()
            .map_err(|err| OutputError::Sink(format!("failed to parse AWS URL: {err:?}")))?;
        Ok(Self {
            url,
            object_prefix,
            credential: key.clone(),
            collection_id: config.collection_id,
            compress: config.compress,
            log_file,
            url_style: UrlStyle::VirtualHost,
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
        for attempt in 0..max_attempts {
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
                        "[forensics] Non-OK response for upload on attempt {attempt}: {:?}",
                        response.text()
                    );
                }
                Err(err) => {
                    log::error!("[forensics] Failed to upload on attempt {attempt}: {err:?}");
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

        for attempt in 0..max_attempts {
            let result = client.post(url.as_str()).body(action.clone().body()).send();
            match result {
                Ok(response) if response.status() == StatusCode::OK => {
                    if response
                        .text()
                        .unwrap_or_default()
                        .contains("Internal Error")
                    {
                        error!(
                            "[forensics] OK response on final upload but the response contained an error for attempt {attempt}"
                        );
                        continue;
                    }
                    return Ok(());
                }
                Ok(response) => {
                    warn!(
                        "[forensics] Non-OK response on attempt {attempt} for final upload : {response:?}"
                    );
                }
                Err(err) => {
                    error!("[forensics] Final upload failed on attempt {attempt}: {err:?}");
                }
            }
        }
        Err(OutputError::Sink(String::from(
            "max attempts reached for AWS upload",
        )))
    }

    /// Construct the full path for our upload
    fn object_path(&self, filename: &str) -> String {
        format!("{}/{filename}", self.object_prefix)
    }

    /// Construct the filename for our upload
    fn construct_filename(&self, artifact_name: &str, extension: &str) -> String {
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
        let bucket = Bucket::new(self.url.clone(), self.url_style, info.bucket, info.region)
            .map_err(|err| {
                OutputError::Sink(format!("failed to initialize AWS bucket: {err:?}"))
            })?;

        let creds = Credentials::new(info.key, info.secret);
        // Valid for one hour
        let duration = Duration::from_secs(3600);
        let action = CreateMultipartUpload::new(&bucket, Some(&creds), object_name);
        let url = action.sign(duration);

        let client = Client::new();
        let max_attempts = 15;

        for attempt in 0..max_attempts {
            let response = client
                .post(url.as_str())
                .send()
                .map_err(|err| OutputError::Sink(format!("failed to start AWS upload: {err:?}")))?;

            if response.status() != StatusCode::OK {
                warn!(
                    "[forensics] Non-200 AWS response on upload start. Attempt {attempt}: {:?}",
                    response.status()
                );
                continue;
            }
            let xml_session = response.text().map_err(|err| {
                OutputError::Sink(format!("failed to get AWS XML start: {err:?}"))
            })?;

            let session = CreateMultipartUpload::parse_response(xml_session).map_err(|err| {
                OutputError::Sink(format!("failed to parse AWS XML session: {err:?}"))
            })?;

            let setup = AwsSetup {
                bucket,
                creds,
                session,
            };

            return Ok(setup);
        }
        Err(OutputError::Sink(String::from(
            "max attempts reached for AWS setup",
        )))
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
            artifact_name,
            OutputLocation::Remote(upload_filename),
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
        Ok(OutputHandle::report(OutputLocation::Remote(upload_report)))
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
        self.upload_bytes(&object_log, data, "text/plain")?;
        let _ = remove_file(&self.log_file);

        Ok(())
    }
}

#[cfg(test)]
impl AwsSink {
    pub(crate) fn with_url_style(mut self, url_style: UrlStyle) -> Self {
        self.url_style = url_style;
        self
    }
}

#[cfg(test)]
mod tests {
    use crate::output::{
        context::CollectionContext,
        error::OutputError,
        report::CollectionReport,
        sink::{aws::AwsSink, output_sink::OutputSink},
    };
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use httpmock::{
        Method::{POST, PUT},
        MockServer,
    };
    use rusty_s3::UrlStyle;
    use std::{io::Write, path::PathBuf};

    fn aws_config(port: u16) -> OutputConfig {
        OutputConfig {
            name: String::from("test"),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            directory: PathBuf::from("./tmp"),
            destination: OutputDestination::Aws,
            format: OutputFormat::Csv,
            // Fake keys created at https://canarytokens.org/generate
            api_key: Some(String::from(
                "ewogICAgImJ1Y2tldCI6ICJibGFoIiwKICAgICJzZWNyZXQiOiAicGtsNkFpQWFrL2JQcEdPenlGVW9DTC96SW1hSEoyTzVtR3ZzVWxSTCIsCiAgICAia2V5IjogIkFLSUEyT0dZQkFINlRPSUFVSk1SIiwKICAgICJyZWdpb24iOiAidXMtZWFzdC0yIgp9",
            )),
            url: Some(format!("http://127.0.0.1:{port}")),
            ..Default::default()
        }
    }

    #[test]
    fn test_aws_sink() {
        let server = MockServer::start();
        let port = server.port();
        let config = aws_config(port);
        let mut sink = AwsSink::new(&config)
            .unwrap()
            .with_url_style(UrlStyle::Path);

        assert!(
            sink.construct_filename("test", "csv")
                .starts_with("./tmp/test/test_")
        );

        let result = sink.decode_creds().unwrap();
        assert_eq!(result.region, "us-east-2");
        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200).body(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
            <InitiateMultipartUploadResult>
            <Bucket>mybucket</Bucket>
            <Key>mykey</Key>
            <UploadId>whatever</UploadId>
         </InitiateMultipartUploadResult>",
            );
        });
        let mock_me_put = server.mock(|when, then| {
            when.method(PUT);
            then.status(200).header("ETAG", "whatever");
        });

        let mut encode = |writer: &mut dyn Write| {
            writer.write_all(br#"{"pid":1}"#)?;
            writer.write_all(b"\n")?;
            Ok(1)
        };

        sink.write_artifact("artifact_name", "jsonl", "application/jsonl", &mut encode)
            .unwrap();

        let context = CollectionContext::new(&config, PathBuf::new());
        let report = CollectionReport::new(&config, &context, Vec::new(), Vec::new());
        sink.write_report(&report).unwrap();
        sink.create_log_file().unwrap();
        sink.finalize().unwrap();
        mock_me.assert_calls(6);
        mock_me_put.assert_calls(3);
    }

    #[test]
    fn test_aws_create_session() {
        let server = MockServer::start();
        let port = server.port();
        let config = aws_config(port);
        let sink = AwsSink::new(&config)
            .unwrap()
            .with_url_style(UrlStyle::Path);

        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200).body(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
            <InitiateMultipartUploadResult>
            <Bucket>mybucket</Bucket>
            <Key>mykey</Key>
            <UploadId>whatever</UploadId>
         </InitiateMultipartUploadResult>",
            );
        });
        let creds = sink.decode_creds().unwrap();
        let session = sink.create_upload_session(creds, "test").unwrap();
        assert!(session.bucket.base_url().as_str().starts_with("http://127"));
        mock_me.assert_calls(1);
    }

    #[test]
    fn test_aws_no_response() {
        let server = MockServer::start();
        let port = server.port();
        let config = aws_config(port);
        let sink = AwsSink::new(&config)
            .unwrap()
            .with_url_style(UrlStyle::Path);

        let err = sink
            .upload_bytes("test", vec![0, 0, 0], "test")
            .unwrap_err();

        assert!(
            matches!(err, OutputError::Sink(value) if value == "max attempts reached for AWS setup")
        );
    }

    #[test]
    fn test_aws_upload_bytes() {
        let server = MockServer::start();
        let port = server.port();
        let config = aws_config(port);
        let sink = AwsSink::new(&config)
            .unwrap()
            .with_url_style(UrlStyle::Path);

        let mock_me = server.mock(|when, then| {
            when.method(POST);
            then.status(200).body(
                "<?xml version=\"1.0\" encoding=\"UTF-8\"?>
            <InitiateMultipartUploadResult>
            <Bucket>mybucket</Bucket>
            <Key>mykey</Key>
            <UploadId>whatever</UploadId>
         </InitiateMultipartUploadResult>",
            );
        });
        let mock_me_put = server.mock(|when, then| {
            when.method(PUT);
            then.status(200).header("ETAG", "whatever");
        });
        sink.upload_bytes("test", vec![0, 0, 0, 0], "application/jsonl")
            .unwrap();

        mock_me.assert_calls(2);
        mock_me_put.assert_calls(1);
    }
}
