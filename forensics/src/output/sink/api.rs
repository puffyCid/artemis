use crate::{
    output::{
        error::{OutputError, OutputResult},
        sink::{
            output_handle::{OutputHandle, OutputLocation},
            output_sink::{LogOutput, OutputSink},
        },
    },
    structs::toml::OutputConfig,
    utils::uuid::generate_uuid,
};
use flate2::{Compression, write::GzEncoder};
use log::{error, warn};
use reqwest::{
    StatusCode,
    blocking::{Client, multipart},
};
use std::{
    fs::{File, create_dir_all, read, remove_file},
    path::PathBuf,
    thread::sleep,
    time::Duration,
};

/// A data Sink representing the API pipeline flow
#[derive(Debug)]
pub(crate) struct ApiSink {
    /// Full URL to API server
    url: String,
    /// Local log file we are using to log any issues during the Artemis execution
    log_file: PathBuf,
    /// Collection ID for the Artemis execution
    collection_id: u64,
    /// Collection name
    name: String,
    /// Unique ID for the endpoint
    endpoint_id: String,
}

impl ApiSink {
    /// Create a API sink and construct the URL target for the uploads
    pub(crate) fn new(config: &OutputConfig) -> OutputResult<Self> {
        let url = match &config.url {
            Some(result) => result,
            None => return Err(OutputError::Sink(String::from("no API URL provided"))),
        };

        // Local directory to store log file. Artemis logs issues locally. Once artifact and report uploads are done
        // The log file is then uploaded. The log file is uploaded last
        let log_file = config.directory.join(&config.name);
        Ok(Self {
            url: url.clone(),
            collection_id: config.collection_id,
            endpoint_id: config.endpoint_id.clone(),
            name: config.name.clone(),
            log_file,
        })
    }

    /// Start the upload process to API server
    fn upload_bytes(
        &self,
        data: Vec<u8>,
        mime_type: &str,
        compress: bool,
        filename: &str,
    ) -> OutputResult<()> {
        let client = Client::new();
        let max_attempts = 15;
        let pause = 8;

        for attempt in 0..max_attempts {
            let mut builder = client
                .post(&self.url)
                .header("x-artemis-endpoint_id", &self.endpoint_id)
                .header("x-artemis-collection_id", self.collection_id)
                .header("x-artemis-collection_name", &self.name)
                .header("accept", "application/json")
                .header("Content-Type", mime_type);

            let mut part = multipart::Part::bytes(data.clone());
            part = part.file_name(filename.to_string());
            if compress {
                builder = builder.header("Content-Encoding", "gzip");
            }
            if filename.ends_with(".log") {
                // The last two uploads for collections are just plaintext log files
                part = part.mime_str("text/plain").unwrap();
            } else if filename.ends_with(".jsonl.gz") {
                // Should be safe to unwrap?
                part = part.mime_str("application/jsonl").unwrap();
            } else {
                // Should be safe to unwrap?
                part = part.mime_str("application/json").unwrap();
            }

            let form = multipart::Form::new().part("artemis-upload", part);
            builder = builder.multipart(form);
            let result = builder.send();

            match result {
                Ok(response) if response.status() == StatusCode::OK => return Ok(()),
                Ok(response) => warn!(
                    "[forensics] Non-OK response from server: {:?}",
                    response.status()
                ),
                Err(err) => {
                    error!("[forensics] Failed to upload data to API. Error: {err:?}");
                }
            }
            let jitter = fastrand::usize(..11);

            let backoff = pause * attempt + jitter;
            // Pause between each attempt
            sleep(Duration::from_secs(backoff as u64));
        }
        Err(OutputError::Sink(String::from(
            "max attempts reached for API upload",
        )))
    }

    /// Return the log file we are logging to
    fn log_filename(&self) -> String {
        format!("artemis_{}_{}.jsonl", self.collection_id, generate_uuid())
    }
}

impl OutputSink for ApiSink {
    fn write_artifact(
        &mut self,
        artifact_name: &str,
        extension: &str,
        mime_type: &str,
        encode: &mut dyn FnMut(
            &mut dyn std::io::prelude::Write,
        ) -> crate::output::error::OutputResult<usize>,
    ) -> crate::output::error::OutputResult<super::output_handle::OutputHandle> {
        let mut gzip = GzEncoder::new(Vec::new(), Compression::default());
        let record_count = encode(&mut gzip)?;
        let data = gzip.finish()?;
        let uuid = generate_uuid();

        // Only JSONL and compress API uploads are supported right now
        let filename = format!("{artifact_name}_{uuid}.jsonl.gz");
        self.upload_bytes(data, mime_type, true, &filename)?;

        Ok(OutputHandle::artifact(
            artifact_name,
            OutputLocation::Remote(self.url.clone()),
            record_count,
            extension,
            true,
        ))
    }

    fn write_report(
        &mut self,
        report: &crate::output::report::CollectionReport,
    ) -> crate::output::error::OutputResult<super::output_handle::OutputHandle> {
        let data = serde_json::to_vec(report)?;
        let filename = format!("report_{}.json", generate_uuid());

        self.upload_bytes(data, "application/json", false, &filename)?;
        Ok(OutputHandle::report(OutputLocation::Remote(
            self.url.clone(),
        )))
    }

    fn create_log_file(
        &mut self,
    ) -> crate::output::error::OutputResult<super::output_sink::LogOutput> {
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
        let data = read(&self.log_file).map_err(|err| OutputError::io_path(&self.log_file, err))?;
        self.upload_bytes(data, "text/plain", false, filename)?;
        let _ = remove_file(&self.log_file);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::output::{error::OutputError, sink::api::ApiSink};
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use httpmock::{Method::POST, MockServer};
    use serde_json::json;
    use std::path::PathBuf;

    fn api_config(port: u16) -> OutputConfig {
        OutputConfig {
            name: String::from("test"),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            directory: PathBuf::from("./tmp"),
            destination: OutputDestination::Api,
            format: OutputFormat::Jsonl,
            url: Some(format!("http://127.0.0.1:{port}")),
            ..Default::default()
        }
    }

    #[test]
    fn test_api_sink() {
        let server = MockServer::start();
        let port = server.port();
        let config = api_config(port);

        let sink = ApiSink::new(&config).unwrap();
        assert!(sink.url.contains("http://127.0.0.1"));
        assert!(!sink.log_file.display().to_string().is_empty());
        assert!(sink.log_filename().starts_with("artemis_"));
        let mock_me = server.mock(|when, then| {
            when.method(POST).header("x-artemis-endpoint_id", "abcd");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({ "message": "ok" }));
        });

        sink.upload_bytes(vec![0, 0, 0, 0, 0], "applicstion/jsonl", true, "test")
            .unwrap();
        mock_me.assert();
    }

    #[test]
    fn test_api_sink_bad_url() {
        let server = MockServer::start();
        let port = server.port();
        let mut config = api_config(port);
        config.url = None;

        let sink = ApiSink::new(&config).unwrap_err();
        assert!(matches!(sink, OutputError::Sink(value) if value == "no API URL provided"))
    }
}
