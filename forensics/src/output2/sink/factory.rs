use crate::output2::{
    config::{OutputConfig, OutputDestination},
    error::{OutputError, OutputResult},
    report::CollectionReport,
    sink::{
        local::LocalSink,
        output_handle::OutputHandle,
        output_sink::{LogOutput, OutputSink},
    },
};

pub(crate) enum Sink {
    Local(LocalSink),
}

impl Sink {
    pub(crate) fn write_artifact(
        &mut self,
        artifact_name: &str,
        extension: &str,
        mime_type: &str,
        encode: &mut dyn FnMut(&mut dyn std::io::Write) -> OutputResult<usize>,
    ) -> OutputResult<OutputHandle> {
        match self {
            Self::Local(sink) => sink.write_artifact(artifact_name, extension, mime_type, encode),
        }
    }

    pub(crate) fn finalize(&mut self) -> OutputResult<()> {
        match self {
            Self::Local(sink) => sink.finalize(),
        }
    }

    pub(crate) fn create_log_file(&mut self) -> OutputResult<LogOutput> {
        match self {
            Self::Local(sink) => sink.create_log_file(),
        }
    }

    pub(crate) fn write_report(&mut self, report: &CollectionReport) -> OutputResult<OutputHandle> {
        match self {
            Self::Local(sink) => sink.write_report(report),
        }
    }
}

pub(crate) fn build_sink(config: &OutputConfig) -> OutputResult<Sink> {
    match config.destination {
        OutputDestination::Local => Ok(Sink::Local(LocalSink::new(config)?)),
        _ => Err(OutputError::UnsupportedDestination(format!(
            "{:?}",
            config.destination
        ))),
    }
}
