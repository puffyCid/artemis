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
use reqwest::{StatusCode, blocking::Client};
use std::{
    fs::{File, create_dir_all, read, remove_file},
    io::Write,
    path::PathBuf,
};
use tracing::error;

/// A data Sink representing the Azure pipeline flow
pub(crate) struct AzureSink {
    /// Full URL to Azure Bucket
    url: String,
    /// Full object that we upload our data too. Contains directory and name of out collection
    object_prefix: String,
    /// Local log file we are using to log any issues during the Artemis execution
    log_file: PathBuf,
    /// Collection ID for the Artemis execution
    collection_id: u64,
    /// Whether to compress the results with gzip
    compress: bool,
}

impl AzureSink {
    /// Create a Azure sink and construct the URL target for the uploads
    pub(crate) fn new(config: &OutputConfig) -> OutputResult<Self> {
        let url = match &config.url {
            Some(result) => result,
            None => return Err(OutputError::Sink(String::from("no Azure bucket provided"))),
        };

        // Full path that we upload data to. Our directory and collection name will be folders in Azure
        // This mimics what Artemis does when writing to local disk
        let object_prefix = format!("{}/{}", config.directory.display(), config.name);

        // Local directory to store log file. Artemis logs issues locally. Once artifact and report uploads are done
        // The log file is then uploaded. The log file is uploaded last
        let log_file = config.directory.join(&config.name);
        Ok(Self {
            url: url.clone(),
            object_prefix,
            collection_id: config.collection_id,
            compress: config.compress,
            log_file,
        })
    }

    /// Start the upload process to Azure
    fn upload_bytes(&self, object_name: &str, data: Vec<u8>, mime_type: &str) -> OutputResult<()> {
        let client = Client::new();
        let max_attempts = 15;
        let azure_url = self.compose_url(object_name)?;

        for attempt in 0..max_attempts {
            let mut builder = client
                .put(&azure_url)
                .header("Content-Type", mime_type)
                .header("Content-Length", data.len())
                .header("x-ms-version", "2019-12-12");

            if !azure_url.contains("&comp=") {
                builder = builder.header("x-ms-blob-type", "BlockBlob");
            }

            let result = builder.body(data.clone()).send();

            match result {
                Ok(response)
                    if response.status() == StatusCode::OK
                        || response.status() == StatusCode::CREATED =>
                {
                    return Ok(());
                }
                Ok(response) => error!(
                    "Non-OK response from Azure blob storage on {attempt}: {:?}",
                    response.status()
                ),
                Err(err) => error!("Failed to upload to Azure on {attempt}: {err:?}"),
            }
        }

        Err(OutputError::Sink(String::from(
            "max attempts reached for Azure upload",
        )))
    }

    /// Compose the final URL to upload data to Azure
    fn compose_url(&self, full_path: &str) -> OutputResult<String> {
        let Some((base, query)) = self.url.split_once('?') else {
            error!("Unexpected Azure URL provided. Missing '?' delimiter.");
            return Err(OutputError::Sink(String::from("Bad Azure URL length")));
        };

        Ok(format!("{base}/{full_path}?{query}"))
    }

    /// URL encode upload paths to "%2F"
    fn encode_path(path: &str) -> String {
        path.trim_matches('/').replace('/', "%2F")
    }

    /// Encode our uploaded filenames
    fn object_path(&self, filename: &str) -> String {
        format!(
            "{}%2F{filename}",
            AzureSink::encode_path(&self.object_prefix)
        )
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

    /// URL decode upload paths to "/"
    fn remote_location(object_prefix: &str) -> String {
        object_prefix.replace("%2F", "/")
    }

    /// Return the log file we are logging to
    fn log_filename(&self) -> String {
        format!("artemis_{}_{}.jsonl", self.collection_id, generate_uuid())
    }
}

impl OutputSink for AzureSink {
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
            OutputLocation::Remote(AzureSink::remote_location(&upload_filename)),
            record_count,
            extension,
            self.compress,
        ))
    }

    fn write_report(
        &mut self,
        report: &crate::output::report::CollectionReport,
    ) -> OutputResult<super::output_handle::OutputHandle> {
        let filename = format!("report_{}.json", generate_uuid());
        let upload_report = self.object_path(&filename);
        let data = serde_json::to_vec(report)?;

        self.upload_bytes(&upload_report, data, "application/json")?;
        Ok(OutputHandle::report(OutputLocation::Remote(
            AzureSink::remote_location(&upload_report),
        )))
    }

    fn create_log_file(&mut self) -> OutputResult<super::output_sink::LogOutput> {
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
mod tests {
    use crate::output::sink::azure::AzureSink;
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use httpmock::{Method::PUT, MockServer};
    use std::path::PathBuf;

    fn azure_config(port: u16) -> OutputConfig {
        OutputConfig {
            name: String::from("test"),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            directory: PathBuf::from("./tmp"),
            destination: OutputDestination::Azure,
            format: OutputFormat::Csv,
            url: Some(format!(
                "http://127.0.0.1:{port}/mycontainername?sp=rcw&st=2023-06-14T03:00:40Z&se=2023-06-14T11:00:40Z&skoid=asdfasdfas-asdfasdfsadf-asdfsfd-sadf"
            )),
            ..Default::default()
        }
    }

    #[test]
    fn test_azure_sink() {
        let server = MockServer::start();
        let port = server.port();
        let config = azure_config(port);

        let sink = AzureSink::new(&config).unwrap();
        assert!(sink.url.contains("http://127.0.0.1"));
        assert!(!sink.log_file.display().to_string().is_empty());
        assert!(sink.log_filename().starts_with("artemis_"));
        assert_eq!(AzureSink::encode_path("test/test"), "test%2Ftest");
        assert!(
            sink.compose_url("test")
                .unwrap()
                .contains("mycontainername/test?")
        );

        let mock_me = server.mock(|when, then| {
            when.method(PUT);
            then.status(200)
                .header("Last-Modified", "2023-06-14 12:00:00")
                .header("Content-MD5", "sQqNsWTgdUEFt6mb5y4/5Q==");
        });

        sink.upload_bytes(
            &sink.construct_filename("test", "jsonl"),
            vec![0, 0, 0, 0],
            "application/csv",
        )
        .unwrap();

        mock_me.assert();
    }
}
