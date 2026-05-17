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
        self.artifact_runs.push(ArtifactRunReport::new(
            artifact_name,
            artifact_options_hash,
            handle.record_count,
            vec![handle.location_string()],
            "completed",
        ));

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
            0,
            Vec::new(),
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
}

fn log_level(level: Option<&str>) -> LevelFilter {
    match level.unwrap_or("warn").to_ascii_uppercase().as_str() {
        "error" => LevelFilter::Error,
        "warn" => LevelFilter::Warn,
        "info" => LevelFilter::Info,
        "debug" => LevelFilter::Debug,
        "trace" => LevelFilter::Trace,
        _ => LevelFilter::Warn,
    }
}
