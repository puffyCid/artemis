use log::LevelFilter;
use simplelog::{Config, WriteLogger};

use crate::output2::{
    config::OutputConfig,
    context::CollectionContext,
    encoder::{artifact_encoder::Encoder, factory::build_encoder},
    error::{OutputError, OutputResult},
    record::RecordStream,
    report::{ArtifactRunReport, CollectionReport},
    sink::factory::{Sink, build_sink},
};

pub(crate) struct OutputManager {
    config: OutputConfig,
    context: CollectionContext,
    encoder: Encoder,
    sink: Sink,
    artifacts: Vec<String>,
    artifact_runs: Vec<ArtifactRunReport>,
}

impl OutputManager {
    pub(crate) fn new(config: OutputConfig, start_time: u64) -> OutputResult<Self> {
        let encoder = build_encoder(&config);
        let mut sink = build_sink(&config)?;

        let log_output = sink.create_log_file()?;
        let log_path = log_output.path.clone();

        WriteLogger::init(
            log_level(config.logging.as_deref()),
            Config::default(),
            log_output.file,
        )
        .map_err(|err| OutputError::Logger(err.to_string()))?;

        let context = CollectionContext::new(&config, start_time, log_path);
        Ok(Self {
            config,
            context,
            encoder,
            sink,
            artifacts: Vec::new(),
            artifact_runs: Vec::new(),
        })
    }

    pub(crate) fn write_artifact(
        &mut self,
        artifact_name: &str,
        artifact_options_hash: String,
        records: &mut dyn RecordStream,
    ) -> OutputResult<()> {
        let artifact_context = self.context.artifact(artifact_name);
        let handle = self.sink.write_artifact(
            artifact_name,
            self.encoder.extension(),
            self.encoder.mime_type(),
            &mut |writer| self.encoder.encode(records, writer, &artifact_context),
        )?;

        self.artifacts.push(artifact_name.to_string());
        self.record_completed_artifact_output(
            artifact_name,
            artifact_options_hash,
            handle.location_string(),
            handle.record_count,
        );

        Ok(())
    }

    pub(crate) fn write_failed_artifact(
        &mut self,
        artifact_name: &str,
        artifact_options_hash: String,
    ) {
        self.artifacts.push(artifact_name.to_string());
        self.artifact_runs.push(ArtifactRunReport::new(
            artifact_name,
            artifact_options_hash,
            Vec::new(),
            0,
            "failed",
        ));
    }

    pub(crate) fn finalize(mut self) -> OutputResult<()> {
        let report = CollectionReport::new(
            &self.config,
            &self.context,
            self.artifacts,
            self.artifact_runs,
        );
        self.sink.write_report(&report)?;
        self.sink.finalize()
    }

    fn record_completed_artifact_output(
        &mut self,
        artifact_name: &str,
        artifact_options_hash: String,
        output_file: String,
        record_count: usize,
    ) {
        if let Some(run) = self.artifact_runs.iter_mut().find(|run| {
            run.name == artifact_name && run.artifact_options_hash == artifact_options_hash
        }) {
            run.add_output_file(output_file, record_count);
            return;
        }
        self.artifact_runs.push(ArtifactRunReport::new(
            artifact_name,
            artifact_options_hash,
            vec![output_file],
            record_count,
            "completed",
        ));
    }
}

fn log_level(level: Option<&str>) -> LevelFilter {
    match level.unwrap_or("warn").to_ascii_lowercase().as_str() {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Warn,
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        output2::{
            config::{OutputConfig, OutputDestination, OutputFormat},
            manager::OutputManager,
            record::{JsonRecord, Record, VecRecordStream},
        },
        utils::time::time_now,
    };
    use serde_json::Map;
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

        let mut manage = OutputManager::new(config, time_now()).unwrap();
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
            .write_artifact("files", String::from("md5"), &mut records)
            .unwrap();

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
            } else if name.starts_with("artemis_") && name.ends_with(".log") {
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
        assert_eq!(report["artifact_runs"][0]["artifact_options_hash"], "md5");
        assert_eq!(report["artifact_runs"][0]["output_count"], 1);
        assert_eq!(report["artifact_runs"][0]["record_count"], 2);
        assert_eq!(report["artifact_runs"][0]["status"], "completed");
    }
}
