use crate::{
    output::{
        context::{ArtifactContext, CollectionContext},
        encoder::{
            artifact_encoder::{Encoder, EncoderMode, StreamWriter},
            factory::build_encoder,
        },
        error::OutputResult,
        record::RecordStream,
        report::{ArtifactRunReport, CollectionReport, hash_artifact_options},
        sink::{
            factory::{Sink, build_sink},
            output_handle::OutputHandle,
        },
    },
    structs::toml::OutputConfig,
};
use serde::Serialize;
use serde_json::Value;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::{fmt::layer, layer::SubscriberExt, util::SubscriberInitExt};

#[cfg(feature = "boa")]
use crate::output::filter::js::JsFilterRecordStream;

/// A structure that supports outputting forensic data based on `OutputConfig`
pub(crate) struct OutputManager {
    /// Configuration to to control how to output data
    pub(crate) config: OutputConfig,
    /// Artemis runtime collection context
    context: CollectionContext,
    /// Output format encoder
    encoder: Encoder,
    /// Destination to write forensic data
    sink: Sink,
    /// Array of artifacts from Artemis collection
    artifacts: Vec<String>,
    /// Array of artifacts collected from the Artemis execution
    pub(crate) artifact_runs: Vec<ArtifactRunReport>,
    pub(crate) filter: bool,
    active_stream: Option<ActiveStream>,
}

struct ActiveStream {
    artifact_name: String,
    artifact_options_hash: String,
    artifact_options: Value,
    output_file: String,
    record_count: usize,
    writer: StreamWriter,
}

impl OutputManager {
    /// Create a manager to output forensic data
    pub(crate) fn new(config: OutputConfig) -> OutputResult<Self> {
        let encoder = build_encoder(&config);
        let mut sink = build_sink(&config)?;

        let log_output = sink.create_log_file()?;
        let log_path = log_output.path.clone();

        let _ = tracing_subscriber::registry()
            .with(
                layer()
                    .json()
                    .with_file(true)
                    .with_line_number(true)
                    .with_target(false)
                    .flatten_event(true)
                    .with_writer(log_output.file),
            )
            .with(log_level(config.logging.as_deref()))
            .try_init();

        let context = CollectionContext::new(&config, log_path);
        Ok(Self {
            config,
            context,
            encoder,
            sink,
            artifacts: Vec::new(),
            artifact_runs: Vec::new(),
            filter: false,
            active_stream: None,
        })
    }

    /// Write a forensic artifact result
    pub(crate) fn write_artifact<T: Serialize>(
        &mut self,
        artifact_name: &str,
        artifact_options: &T,
        records: &mut dyn RecordStream,
    ) -> OutputResult<()> {
        match self.encoder.encoder_mode() {
            EncoderMode::Chunked => {
                let handle = self.write(artifact_name, records)?;

                if !self.artifacts.iter().any(|name| name == artifact_name) {
                    self.artifacts.push(artifact_name.to_string());
                }
                self.record_completed_artifact_output(
                    artifact_name,
                    artifact_options,
                    handle.location_string(),
                    handle.record_count,
                );

                Ok(())
            }
            EncoderMode::Streamed => self.write_stream(artifact_name, artifact_options, records),
        }
    }

    /// Write a failed artifact run
    pub(crate) fn write_failed_artifact<T: Serialize>(
        &mut self,
        artifact_name: &str,
        artifact_options: &T,
    ) {
        if !self.artifacts.iter().any(|name| name == artifact_name) {
            self.artifacts.push(artifact_name.to_string());
        }
        self.artifact_runs.push(ArtifactRunReport::new(
            artifact_name,
            artifact_options,
            Vec::new(),
            0,
            "failed",
        ));
    }

    /// Complete a Artemis collection execution
    pub(crate) fn finalize(mut self) -> OutputResult<()> {
        // Complete any active writer stream
        self.finish_stream()?;
        let report = CollectionReport::new(
            &self.config,
            &self.context,
            self.artifacts,
            self.artifact_runs,
        );
        self.sink.write_report(&report)?;
        self.sink.finalize()
    }

    /// Track artifact collected from Artemis execution
    fn record_completed_artifact_output<T: Serialize>(
        &mut self,
        artifact_name: &str,
        artifact_options: &T,
        output_file: String,
        record_count: usize,
    ) {
        let hash = hash_artifact_options(&artifact_options).unwrap_or_default();
        // Only track unique artifacts per Artemis collection
        // If a user collects a process listing twice in a single Artemis collection
        // We only record `Processes` artifact once instead of twice
        if let Some(run) = self
            .artifact_runs
            .iter_mut()
            .find(|run| run.name == artifact_name && run.artifact_options_hash == hash)
        {
            run.add_output_file(output_file, record_count);
            return;
        }
        // Track each artifact run event
        self.artifact_runs.push(ArtifactRunReport::new(
            artifact_name,
            artifact_options,
            vec![output_file],
            record_count,
            "completed",
        ));
    }

    /// Write artifact records to our configured destination `Sink`
    fn write(
        &mut self,
        artifact_name: &str,
        records: &mut dyn RecordStream,
    ) -> OutputResult<OutputHandle> {
        let artifact_context = self.context.artifact(
            artifact_name,
            &self.config.start_time_filter,
            &self.config.end_time_filter,
        );
        // If boa is enabled and we have a filter script
        // Filter records before writing them to Sink
        #[cfg(feature = "boa")]
        if self.filter
            && let Some(script) = &self.config.filter_script
        {
            // User should give us a name. But if we do not have one
            // Use `UnknownFilterScript` as default
            let filter_name = self
                .config
                .filter_name
                .as_deref()
                .unwrap_or("UnknownFilterScript");
            let mut filtered_records = JsFilterRecordStream::new(
                records,
                script,
                artifact_name,
                filter_name,
                &self.context,
            )?;

            let handle = self.sink.write_artifact(
                artifact_name,
                self.encoder.extension(),
                self.encoder.mime_type(),
                &mut |writer| {
                    self.encoder
                        .encode(&mut filtered_records, writer, &artifact_context)
                },
            )?;

            return Ok(handle);
        }

        self.sink.write_artifact(
            artifact_name,
            self.encoder.extension(),
            self.encoder.mime_type(),
            &mut |writer| self.encoder.encode(records, writer, &artifact_context),
        )
    }

    /// Write artifact records to our configured destination `Sink`
    ///
    /// This writer streams the data to a single on disk
    fn write_stream<T: Serialize>(
        &mut self,
        artifact_name: &str,
        artifact_options: &T,
        records: &mut dyn RecordStream,
    ) -> OutputResult<()> {
        let artifact_context = self.context.artifact(
            artifact_name,
            &self.config.start_time_filter,
            &self.config.end_time_filter,
        );

        // If boa is enabled and we have a filter script
        // Filter records before writing them to Sink
        #[cfg(feature = "boa")]
        if self.filter
            && let Some(script) = &self.config.filter_script
        {
            // User should give us a name. But if we do not have one
            // Use `UnknownFilterScript` as default
            let filter_name = self
                .config
                .filter_name
                .as_deref()
                .unwrap_or("UnknownFilterScript");
            let mut filtered_records = JsFilterRecordStream::new(
                records,
                script,
                artifact_name,
                filter_name,
                &self.context,
            )?;

            return self.write_stream_records(
                artifact_name,
                artifact_options,
                &mut filtered_records,
                &artifact_context,
            );
        }

        self.write_stream_records(artifact_name, artifact_options, records, &artifact_context)
    }

    /// Write records to single file on disk
    fn write_stream_records<T: Serialize>(
        &mut self,
        artifact_name: &str,
        artifact_options: &T,
        records: &mut dyn RecordStream,
        artifact_context: &ArtifactContext,
    ) -> OutputResult<()> {
        let options_hash = hash_artifact_options(artifact_options)?;
        let should_finish = self.active_stream.as_ref().is_some_and(|act| {
            act.artifact_name != artifact_name || act.artifact_options_hash != options_hash
        });

        if should_finish {
            self.finish_stream()?;
        }

        if let Some(active) = self.active_stream.as_mut() {
            let count = active.writer.write_records(records, artifact_context)?;
            active.record_count += count;
            return Ok(());
        }

        let target = self
            .sink
            .stream_artifact(artifact_name, self.encoder.extension())?;

        let output_file = target.path.display().to_string();
        let open = self
            .encoder
            .encode_stream(target, records, artifact_context)?;

        self.active_stream = Some(ActiveStream {
            artifact_name: artifact_name.to_string(),
            artifact_options_hash: options_hash,
            artifact_options: serde_json::to_value(artifact_options).unwrap_or_default(),
            output_file,
            record_count: open.record_count,
            writer: open.writer,
        });
        if !self.artifacts.iter().any(|name| name == artifact_name) {
            self.artifacts.push(artifact_name.to_string());
        }
        Ok(())
    }

    /// Complete streaming to file on disk
    fn finish_stream(&mut self) -> OutputResult<()> {
        let Some(output) = self.active_stream.take() else {
            return Ok(());
        };
        let ActiveStream {
            artifact_name,
            artifact_options_hash,
            artifact_options,
            output_file,
            record_count,
            writer,
        } = output;

        writer.finish()?;
        self.record_complete_stream(
            artifact_name,
            artifact_options_hash,
            artifact_options,
            output_file,
            record_count,
        );
        Ok(())
    }

    /// Update our artifact run report every time we complete writing records to disk
    fn record_complete_stream(
        &mut self,
        artifact_name: String,
        artifact_option_hash: String,
        artifact_options: Value,
        output_file: String,
        record_count: usize,
    ) {
        if let Some(run) = self.artifact_runs.iter_mut().find(|run| {
            run.name == artifact_name && run.artifact_options_hash == artifact_option_hash
        }) {
            run.add_output_file(output_file, record_count);
            return;
        }

        let mut run = ArtifactRunReport::new(
            &artifact_name,
            &artifact_options,
            vec![output_file],
            record_count,
            "completed",
        );
        run.artifact_options_hash = artifact_option_hash;
        self.artifact_runs.push(run);
    }
}

/// Translate Artemis collection log level to proper `LevelFilter`
fn log_level(level: Option<&str>) -> LevelFilter {
    match level.unwrap_or("warn").to_ascii_lowercase().as_str() {
        "error" => LevelFilter::ERROR,
        "info" => LevelFilter::INFO,
        "debug" => LevelFilter::DEBUG,
        "trace" => LevelFilter::TRACE,
        _ => LevelFilter::WARN,
    }
}

#[cfg(test)]
mod tests {
    use crate::output::{
        manager::OutputManager,
        record::{JsonRecord, Record, ScalarRecord, VecRecordStream},
    };
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use httpmock::{
        Method::{POST, PUT},
        MockServer,
    };
    use serde_json::{Map, json};
    use std::{
        fs::{read_dir, read_to_string},
        path::PathBuf,
    };

    #[test]
    fn test_output_manager() {
        let name = String::from("manager_collection");
        let config = OutputConfig {
            name,
            endpoint_id: String::from("test"),
            collection_id: 0,
            directory: PathBuf::from("./tmp"),
            destination: OutputDestination::Local,
            format: OutputFormat::Jsonl,
            ..Default::default()
        };

        let mut manage = OutputManager::new(config).unwrap();
        let mut first = Map::new();
        first.insert("path".to_string(), "/tmp/one.txt".into());
        first.insert("size".to_string(), 1235.into());
        let mut second = Map::new();
        second.insert("path".to_string(), "/tmp/two.txt".into());
        second.insert("size".to_string(), 5.into());
        let mut records = VecRecordStream::new(vec![
            Record::Json(JsonRecord::new(first)),
            Record::Json(JsonRecord::new(second)),
        ]);

        manage
            .write_artifact(
                "files",
                &json!({"start_path": "./tmp", "depth": 99}),
                &mut records,
            )
            .unwrap();

        manage.write_failed_artifact("made_up_artifact", &String::from("test"));

        manage.finalize().unwrap();

        let output_dir = PathBuf::from("./tmp").join(String::from("manager_collection"));
        assert!(output_dir.exists());

        let mut jsonl_files = Vec::new();
        let mut report_files = Vec::new();
        let mut log_files = Vec::new();
        for entry in read_dir(&output_dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy();
            if name.starts_with("files_") && name.ends_with(".jsonl") {
                jsonl_files.push(path);
            } else if name.starts_with("report_") && name.ends_with(".json") {
                report_files.push(path);
            } else if name.starts_with("artemis_") && name.ends_with(".jsonl") {
                log_files.push(path);
            }
        }
        assert!(!jsonl_files.is_empty());
        assert!(!report_files.is_empty());
        assert!(!log_files.is_empty());
        let jsonl_data = read_to_string(&jsonl_files[0]).unwrap();
        let lines = jsonl_data.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        let first_record: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first_record["path"], "/tmp/one.txt");
        assert_eq!(first_record["size"], 1235);
        assert_eq!(first_record["collection_metadata"]["endpoint_id"], "test");
        assert_eq!(first_record["collection_metadata"]["id"], 0);
        assert_eq!(
            first_record["collection_metadata"]["artifact_name"],
            "files"
        );
        let second_record: serde_json::Value = serde_json::from_str(lines[1]).unwrap();
        assert_eq!(second_record["path"], "/tmp/two.txt");
        assert_eq!(second_record["size"], 5);
        let report_data = read_to_string(&report_files[0]).unwrap();
        let report: serde_json::Value = serde_json::from_str(&report_data).unwrap();
        assert_eq!(report["collection_id"], 0);
        assert_eq!(report["endpoint_id"], "test");
        assert_eq!(report["total_output_files"], 1);
        assert_eq!(report["artifacts"][0], "files");
        assert_eq!(report["artifact_runs"][0]["name"], "files");
        assert_eq!(
            report["artifact_runs"][0]["artifact_options_hash"],
            "2297a2e4d2902655a171ae9b818ce092"
        );
        assert_eq!(report["artifact_runs"][0]["output_count"], 1);
        assert_eq!(report["artifact_runs"][0]["record_count"], 2);
        assert_eq!(report["artifact_runs"][0]["status"], "completed");
    }

    #[test]
    #[cfg(feature = "gcp")]
    fn test_output_manager_gcp() {
        let server = MockServer::start();
        let port = server.port();
        let name = String::from("manager_gcp_collection");
        let config = OutputConfig {
            name,
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
        };

        let mut manage = OutputManager::new(config).unwrap();
        let mut first = Map::new();
        first.insert("path".to_string(), "/tmp/one.txt".into());
        first.insert("size".to_string(), 1235.into());
        let mut second = Map::new();
        second.insert("path".to_string(), "/tmp/two.txt".into());
        second.insert("size".to_string(), 5.into());
        let mut records = VecRecordStream::new(vec![
            Record::Json(JsonRecord::new(first)),
            Record::Json(JsonRecord::new(second)),
        ]);
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

        manage
            .write_artifact(
                "files",
                &json!({"start_path": "./tmp", "depth": 99}),
                &mut records,
            )
            .unwrap();

        manage.write_failed_artifact("madeup", &String::from("nothing matters"));
        manage.finalize().unwrap();

        // 3 uploads:
        // Dummy artifact
        // Failed artifact
        // log file
        mock_me.assert_calls(3);
        mock_me_put.assert_calls(3);
    }

    #[test]
    fn test_output_manager_timeline() {
        let name = String::from("manager_collection_timeline");
        let config = OutputConfig {
            name,
            endpoint_id: String::from("test"),
            collection_id: 0,
            directory: PathBuf::from("./tmp"),
            destination: OutputDestination::Local,
            format: OutputFormat::Timeline,
            ..Default::default()
        };

        let mut manage = OutputManager::new(config).unwrap();
        let mut first = Map::new();
        first.insert("full_path".to_string(), "/tmp/one.txt".into());
        first.insert("arguments".to_string(), "1235".into());
        first.insert("start_time".to_string(), "2026-01-01T00:00:00.000Z".into());
        let mut records = VecRecordStream::new(vec![Record::Json(JsonRecord::new(first))]);

        manage
            .write_artifact(
                "processes",
                &json!({"start_path": "./tmp", "depth": 99}),
                &mut records,
            )
            .unwrap();

        manage.write_failed_artifact("made_up_artifact", &String::from("test"));

        manage.finalize().unwrap();

        let output_dir = PathBuf::from("./tmp").join(String::from("manager_collection_timeline"));
        assert!(output_dir.exists());

        let mut jsonl_files = Vec::new();
        let mut report_files = Vec::new();
        let mut log_files = Vec::new();
        for entry in read_dir(&output_dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy();
            if name.starts_with("processes_") && name.ends_with(".jsonl") {
                jsonl_files.push(path);
            } else if name.starts_with("report_") && name.ends_with(".json") {
                report_files.push(path);
            } else if name.starts_with("artemis_") && name.ends_with(".jsonl") {
                log_files.push(path);
            }
        }
        assert!(!jsonl_files.is_empty());
        assert!(!report_files.is_empty());
        assert!(!log_files.is_empty());
        let jsonl_data = read_to_string(&jsonl_files[0]).unwrap();
        let lines = jsonl_data.lines().collect::<Vec<_>>();
        let first_record: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first_record["full_path"], "/tmp/one.txt");
        assert_eq!(first_record["arguments"], "1235");
        assert_eq!(first_record["collection_metadata"]["endpoint_id"], "test");
        assert_eq!(first_record["collection_metadata"]["id"], 0);
        assert_eq!(
            first_record["collection_metadata"]["artifact_name"],
            "processes"
        );
        assert_eq!(first_record["message"], "/tmp/one.txt 1235");
        assert_eq!(first_record["datetime"], "2026-01-01T00:00:00.000Z");
        assert_eq!(first_record["data_type"], "system:processes:process");
        let report_data = read_to_string(&report_files[0]).unwrap();
        let report: serde_json::Value = serde_json::from_str(&report_data).unwrap();
        assert_eq!(report["collection_id"], 0);
        assert_eq!(report["endpoint_id"], "test");
        assert_eq!(report["total_output_files"], 1);
        assert_eq!(report["artifacts"][0], "processes");
        assert_eq!(report["artifact_runs"][0]["name"], "processes");
        assert_eq!(
            report["artifact_runs"][0]["artifact_options_hash"],
            "2297a2e4d2902655a171ae9b818ce092"
        );
        assert_eq!(report["artifact_runs"][0]["output_count"], 1);
        assert_eq!(report["artifact_runs"][0]["record_count"], 1);
        assert_eq!(report["artifact_runs"][0]["status"], "completed");
    }

    #[test]
    #[cfg(feature = "azure")]
    fn test_output_manager_azure() {
        let server = MockServer::start();
        let port = server.port();
        let name = String::from("manager_azure_collection");
        let config = OutputConfig {
            name,
            endpoint_id: String::from("test"),
            collection_id: 0,
            directory: PathBuf::from("./tmp"),
            destination: OutputDestination::Azure,
            format: OutputFormat::Json,
            url: Some(format!(
                "http://127.0.0.1:{port}/mycontainername?sp=rcw&st=2023-06-14T03:00:40Z&se=2023-06-14T11:00:40Z&skoid=asdfasdfas-asdfasdfsadf-asdfsfd-sadf"
            )),
            ..Default::default()
        };

        let mut manage = OutputManager::new(config).unwrap();
        let mut first = Map::new();
        first.insert("path".to_string(), "/tmp/one.txt".into());
        first.insert("size".to_string(), 1235.into());
        let mut second = Map::new();
        second.insert("path".to_string(), "/tmp/two.txt".into());
        second.insert("size".to_string(), 5.into());
        let mut records = VecRecordStream::new(vec![
            Record::Json(JsonRecord::new(first)),
            Record::Json(JsonRecord::new(second)),
        ]);
        let mock_me = server.mock(|when, then| {
            when.method(PUT);
            then.status(200)
                .header("Last-Modified", "2023-06-14 12:00:00")
                .header("Content-MD5", "sQqNsWTgdUEFt6mb5y4/5Q==");
        });

        manage
            .write_artifact(
                "files",
                &json!({"start_path": "./tmp", "depth": 99}),
                &mut records,
            )
            .unwrap();

        manage.write_failed_artifact("madeup", &String::from("nothing matters"));
        manage.finalize().unwrap();

        // 3 uploads:
        // Dummy artifact
        // Failed artifact
        // log file
        mock_me.assert_calls(3);
    }

    #[test]
    #[cfg(feature = "api")]
    fn test_output_manager_api() {
        let server = MockServer::start();
        let port = server.port();
        let name = String::from("manager_api_collection");
        let config = OutputConfig {
            name,
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            directory: PathBuf::from("./tmp"),
            destination: OutputDestination::Api,
            format: OutputFormat::Jsonl,
            url: Some(format!("http://127.0.0.1:{port}")),
            ..Default::default()
        };

        let mut manage = OutputManager::new(config).unwrap();
        let mut first = Map::new();
        first.insert("path".to_string(), "/tmp/one.txt".into());
        first.insert("size".to_string(), 1235.into());
        let mut second = Map::new();
        second.insert("path".to_string(), "/tmp/two.txt".into());
        second.insert("size".to_string(), 5.into());
        let mut records = VecRecordStream::new(vec![
            Record::Json(JsonRecord::new(first)),
            Record::Json(JsonRecord::new(second)),
        ]);
        let mock_me = server.mock(|when, then| {
            when.method(POST).header("x-artemis-endpoint_id", "abcd");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({ "message": "ok" }));
        });

        manage
            .write_artifact(
                "files",
                &json!({"start_path": "./tmp", "depth": 99}),
                &mut records,
            )
            .unwrap();

        manage.write_failed_artifact("madeup", &String::from("nothing matters"));
        manage.finalize().unwrap();

        // 3 uploads:
        // Dummy artifact
        // Failed artifact
        // log file
        mock_me.assert_calls(3);
    }

    #[test]
    #[cfg(feature = "boa")]
    fn test_output_js_filter() {
        let name = String::from("manager_js_collection");
        let config = OutputConfig {
            name,
            endpoint_id: String::from("test"),
            collection_id: 0,
            directory: PathBuf::from("./tmp"),
            destination: OutputDestination::Local,
            format: OutputFormat::Jsonl,
            filter_name: Some(String::from("test")),
            filter_script: Some(String::from(
                "ZnVuY3Rpb24gbWFpbih2YWx1ZSwgY29udGV4dCkgewogIGlmKHZhbHVlLnBhdGggIT09ICIvdG1wL3R3by50eHQiKSB7CiAgICByZXR1cm4gbnVsbDsKICB9CgogIGNvbnNvbGUubG9nKGBJIGdvdCAke3ZhbHVlLnBhdGh9YCk7CiAgY29uc29sZS5sb2coYENvbnRleHQgaXMgZW5kcG9pbnQgSUQ6ICR7Y29udGV4dC5lbmRwb2ludF9pZH1gKTsKICB2YWx1ZVsibWVzc2FnZSJdID0gIllvdSBnb3QgZmlsdGVyZWQhIjsKICByZXR1cm4gdmFsdWU7Cn0=",
            )),
            ..Default::default()
        };

        let mut manage = OutputManager::new(config).unwrap();
        manage.filter = true;
        let mut first = Map::new();
        first.insert("path".to_string(), "/tmp/one.txt".into());
        first.insert("size".to_string(), 1235.into());
        let mut second = Map::new();
        second.insert("path".to_string(), "/tmp/two.txt".into());
        second.insert("size".to_string(), 5.into());
        let mut records = VecRecordStream::new(vec![
            Record::Json(JsonRecord::new(first)),
            Record::Json(JsonRecord::new(second)),
        ]);

        manage
            .write_artifact(
                "files",
                &json!({"start_path": "./tmp", "depth": 99}),
                &mut records,
            )
            .unwrap();

        manage.finalize().unwrap();
        let output_dir = PathBuf::from("./tmp").join(String::from("manager_js_collection"));
        assert!(output_dir.exists());
        let mut jsonl_files = Vec::new();
        let mut report_files = Vec::new();
        let mut log_files = Vec::new();
        for entry in read_dir(&output_dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy();
            if name.starts_with("files_") && name.ends_with(".jsonl") {
                jsonl_files.push(path);
            } else if name.starts_with("report_") && name.ends_with(".json") {
                report_files.push(path);
            } else if name.starts_with("artemis_") && name.ends_with(".jsonl") {
                log_files.push(path);
            }
        }
        assert!(!jsonl_files.is_empty());
        assert!(!report_files.is_empty());
        assert!(!log_files.is_empty());
        let jsonl_data = read_to_string(&jsonl_files[0]).unwrap();
        let lines = jsonl_data.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 1);
        let first_record: serde_json::Value = serde_json::from_str(lines[0]).unwrap();
        assert_eq!(first_record["path"], "/tmp/two.txt");
        assert_eq!(first_record["size"], 5);
        assert_eq!(first_record["message"], "You got filtered!");
        assert_eq!(first_record["collection_metadata"]["endpoint_id"], "test");
        assert_eq!(first_record["collection_metadata"]["id"], 0);
        assert_eq!(
            first_record["collection_metadata"]["artifact_name"],
            "files"
        );
    }

    #[test]
    #[cfg(feature = "boa")]
    fn test_output_js_async_filter() {
        let name = String::from("manager_js_async_collection");
        let config = OutputConfig {
            name,
            endpoint_id: String::from("test"),
            collection_id: 0,
            directory: PathBuf::from("./tmp"),
            destination: OutputDestination::Local,
            format: OutputFormat::Csv,
            filter_name: Some(String::from("test")),
            filter_script: Some(String::from(
                "YXN5bmMgZnVuY3Rpb24gbWFpbihyZWNvcmQsIGNvbnRleHQpIHsKICBhd2FpdCBQcm9taXNlLnJlc29sdmUoKTsKICBpZihyZWNvcmQucGF0aCAhPT0gIi90bXAvdHdvLnR4dCIpIHsKICAgIHJldHVybiBudWxsOwogIH0KIGNvbnNvbGUubG9nKGBJIGdvdCAke3JlY29yZC5wYXRofWApOwogIGNvbnNvbGUubG9nKGBDb250ZXh0IGlzIGVuZHBvaW50IElEOiAke2NvbnRleHQuZW5kcG9pbnRfaWR9YCk7CiAgcmVjb3JkWyJtZXNzYWdlIl0gPSAiWW91IGdvdCBhc3luYyBmaWx0ZXJlZCEiOwogIHJlY29yZFsiZmlsdGVyZWRfYnkiXSA9IGNvbnRleHQuZmlsdGVyX25hbWU7CiAgcmVjb3JkWyJhc3luY19maWx0ZXIiXSA9IHRydWU7CiAgcmV0dXJuIHJlY29yZDsKfQ==",
            )),
            ..Default::default()
        };

        let mut manage = OutputManager::new(config).unwrap();
        manage.filter = true;
        let mut first = Map::new();
        first.insert("path".to_string(), "/tmp/one.txt".into());
        first.insert("size".to_string(), 1235.into());
        let mut second = Map::new();
        second.insert("path".to_string(), "/tmp/two.txt".into());
        second.insert("size".to_string(), 5.into());
        let mut records = VecRecordStream::new(vec![
            Record::Json(JsonRecord::new(first)),
            Record::Json(JsonRecord::new(second)),
        ]);

        manage
            .write_artifact(
                "files",
                &json!({"start_path": "./tmp", "depth": 99}),
                &mut records,
            )
            .unwrap();

        manage.finalize().unwrap();
        let output_dir = PathBuf::from("./tmp").join(String::from("manager_js_async_collection"));
        assert!(output_dir.exists());
        let mut csv_files = Vec::new();
        let mut report_files = Vec::new();
        let mut log_files = Vec::new();
        for entry in read_dir(&output_dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy();
            if name.starts_with("files_") && name.ends_with(".csv") {
                csv_files.push(path);
            } else if name.starts_with("report_") && name.ends_with(".json") {
                report_files.push(path);
            } else if name.starts_with("artemis_") && name.ends_with(".jsonl") {
                log_files.push(path);
            }
        }
        assert!(!csv_files.is_empty());
        assert!(!report_files.is_empty());
        assert!(!log_files.is_empty());
        let csv_data = read_to_string(&csv_files[0]).unwrap();
        let lines = csv_data.lines().collect::<Vec<_>>();
        assert_eq!(lines.len(), 2);
        assert!(lines[1].contains("You got async filtered!"));
        assert!(lines[1].contains(",true"));
    }

    #[test]
    fn test_output_manager_text() {
        let name = String::from("manager_collection_txt");
        let config = OutputConfig {
            name,
            endpoint_id: String::from("test"),
            collection_id: 0,
            directory: PathBuf::from("./tmp"),
            destination: OutputDestination::Local,
            format: OutputFormat::Text,
            ..Default::default()
        };

        let mut manage = OutputManager::new(config).unwrap();
        let mut records = VecRecordStream::new(vec![
            Record::Scalar(ScalarRecord::Text(String::from("hello boa"))),
            Record::Scalar(ScalarRecord::Integer(100)),
            Record::Scalar(ScalarRecord::Bool(true)),
            Record::Scalar(ScalarRecord::Float(3.14)),
            Record::Null,
        ]);

        manage
            .write_artifact("runtime_text", &json!({"runtime": "boajs"}), &mut records)
            .unwrap();

        manage.finalize().unwrap();

        let output_dir = PathBuf::from("./tmp").join(String::from("manager_collection_txt"));
        assert!(output_dir.exists());
        let mut txt_files = Vec::new();
        let mut report_files = Vec::new();
        for entry in read_dir(&output_dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy();
            if name.starts_with("runtime_text_") && name.ends_with(".txt") {
                txt_files.push(path);
            } else if name.starts_with("report_") && name.ends_with(".json") {
                report_files.push(path);
            }
        }
        assert!(txt_files.len() >= 1);
        let txt_data = read_to_string(&txt_files[0]).unwrap();
        let lines = txt_data.lines().collect::<Vec<_>>();
        assert_eq!(lines, vec!["hello boa", "100", "true", "3.14", "null"]);
        let report_data = read_to_string(&report_files[0]).unwrap();
        let report: serde_json::Value = serde_json::from_str(&report_data).unwrap();
        assert_eq!(report["total_output_files"], 1);
        assert_eq!(report["artifact_runs"][0]["name"], "runtime_text");
        assert_eq!(report["artifact_runs"][0]["record_count"], 5);
        assert_eq!(report["artifact_runs"][0]["status"], "completed");
    }

    #[test]
    fn test_output_manager_xml() {
        let name = String::from("manager_collection");
        let config = OutputConfig {
            name,
            endpoint_id: String::from("test"),
            collection_id: 0,
            directory: PathBuf::from("./tmp"),
            destination: OutputDestination::Local,
            format: OutputFormat::Xml,
            ..Default::default()
        };

        let mut manage = OutputManager::new(config).unwrap();
        let mut first = Map::new();
        first.insert("path".to_string(), "/tmp/one.txt".into());
        first.insert("size".to_string(), 1235.into());
        let mut second = Map::new();
        second.insert("path".to_string(), "/tmp/two.txt".into());
        second.insert("size".to_string(), 5.into());
        let mut records = VecRecordStream::new(vec![
            Record::Json(JsonRecord::new(first)),
            Record::Json(JsonRecord::new(second)),
        ]);

        manage
            .write_artifact(
                "files",
                &json!({"start_path": "./tmp", "depth": 99}),
                &mut records,
            )
            .unwrap();

        manage.write_failed_artifact("made_up_artifact", &String::from("test"));

        manage.finalize().unwrap();

        let output_dir = PathBuf::from("./tmp").join(String::from("manager_collection"));
        assert!(output_dir.exists());

        let mut xml_files = Vec::new();
        let mut report_files = Vec::new();
        let mut log_files = Vec::new();
        for entry in read_dir(&output_dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy();
            if name.starts_with("files_") && name.ends_with(".xml") {
                xml_files.push(path);
            } else if name.starts_with("report_") && name.ends_with(".json") {
                report_files.push(path);
            } else if name.starts_with("artemis_") && name.ends_with(".jsonl") {
                log_files.push(path);
            }
        }
        assert!(!xml_files.is_empty());
        assert!(!report_files.is_empty());
        assert!(!log_files.is_empty());
        let xml_data = read_to_string(&xml_files[0]).unwrap();
        assert!(xml_data.contains("<path>/tmp/one.txt</path>"));
    }

    #[test]
    fn test_output_manager_parquet() {
        let name = String::from("manager_collection_par");
        let config = OutputConfig {
            name,
            endpoint_id: String::from("test"),
            collection_id: 0,
            directory: PathBuf::from("./tmp"),
            destination: OutputDestination::Local,
            format: OutputFormat::Parquet,
            ..Default::default()
        };

        let mut manage = OutputManager::new(config).unwrap();
        let mut first = Map::new();
        first.insert("path".to_string(), "/tmp/one.txt".into());
        first.insert("size".to_string(), 1235.into());
        let mut second = Map::new();
        second.insert("path".to_string(), "/tmp/two.txt".into());
        second.insert("size".to_string(), 5.into());
        let mut records = VecRecordStream::new(vec![
            Record::Json(JsonRecord::new(first)),
            Record::Json(JsonRecord::new(second)),
        ]);

        manage
            .write_artifact(
                "files",
                &json!({"start_path": "./tmp", "depth": 99}),
                &mut records,
            )
            .unwrap();

        manage.finalize().unwrap();

        let output_dir = PathBuf::from("./tmp").join(String::from("manager_collection_par"));
        assert!(output_dir.exists());

        let mut par_files = Vec::new();
        let mut report_files = Vec::new();
        let mut log_files = Vec::new();
        for entry in read_dir(&output_dir).unwrap() {
            let path = entry.unwrap().path();
            let name = path.file_name().unwrap().to_string_lossy();
            if name.starts_with("files_") && name.ends_with(".parquet") {
                par_files.push(path);
            } else if name.starts_with("report_") && name.ends_with(".json") {
                report_files.push(path);
            } else if name.starts_with("artemis_") && name.ends_with(".jsonl") {
                log_files.push(path);
            }
        }
        assert!(!par_files.is_empty());
        assert!(!report_files.is_empty());
        assert!(!log_files.is_empty());
    }
}
