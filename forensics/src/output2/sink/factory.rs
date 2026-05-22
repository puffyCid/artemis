use crate::output2::{
    config::{OutputConfig, OutputDestination},
    error::{OutputError, OutputResult},
    report::CollectionReport,
    sink::{
        aws::AwsSink,
        gcp::GcpSink,
        local::LocalSink,
        output_handle::OutputHandle,
        output_sink::{LogOutput, OutputSink},
    },
};

/// Selected destination for encoded output.
///
/// `Sink` delegates destination writing to the configured output sink.
/// <https://en.wikipedia.org/wiki/Sink_(computing)>
pub(crate) enum Sink {
    /// Write encoded output to local system
    Local(LocalSink),
    Gcp(GcpSink),
    Aws(AwsSink),
}

impl Sink {
    /// Write artifact results to destination
    pub(crate) fn write_artifact(
        &mut self,
        artifact_name: &str,
        extension: &str,
        mime_type: &str,
        encode: &mut dyn FnMut(&mut dyn std::io::Write) -> OutputResult<usize>,
    ) -> OutputResult<OutputHandle> {
        match self {
            Self::Local(sink) => sink.write_artifact(artifact_name, extension, mime_type, encode),
            Self::Gcp(sink) => sink.write_artifact(artifact_name, extension, mime_type, encode),
            Self::Aws(sink) => sink.write_artifact(artifact_name, extension, mime_type, encode),
        }
    }

    /// Finalizes destination-specific output.
    ///
    /// For local output, this may perform final zip compression and cleanup.
    pub(crate) fn finalize(&mut self) -> OutputResult<()> {
        match self {
            Self::Local(sink) => sink.finalize(),
            Self::Gcp(sink) => sink.finalize(),
            Self::Aws(sink) => sink.finalize(),
        }
    }

    /// Initialize the log file
    pub(crate) fn create_log_file(&mut self) -> OutputResult<LogOutput> {
        match self {
            Self::Local(sink) => sink.create_log_file(),
            Self::Gcp(sink) => sink.create_log_file(),
            Self::Aws(sink) => sink.create_log_file(),
        }
    }

    /// Once collection is completed, write a JSON collection report
    pub(crate) fn write_report(&mut self, report: &CollectionReport) -> OutputResult<OutputHandle> {
        match self {
            Self::Local(sink) => sink.write_report(report),
            Self::Gcp(sink) => sink.write_report(report),
            Self::Aws(sink) => sink.write_report(report),
        }
    }
}

/// Setup a `Sink` to send data to selected output destination
pub(crate) fn build_sink(config: &OutputConfig) -> OutputResult<Sink> {
    match config.destination {
        OutputDestination::Local => Ok(Sink::Local(LocalSink::new(config)?)),
        OutputDestination::Gcp => Ok(Sink::Gcp(GcpSink::new(config)?)),
        OutputDestination::Aws => Ok(Sink::Aws(AwsSink::new(config)?)),
        _ => Err(OutputError::UnsupportedDestination(format!(
            "{:?}",
            config.destination
        ))),
    }
}

#[cfg(test)]
mod tests {
    use crate::output2::{
        config::{OutputConfig, OutputFormat},
        manager::OutputManager,
        record::{JsonRecord, Record, VecRecordStream},
    };
    use log::error;
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn test_sink_local_write_artifact() {
        let test = json!({"test":"value"});
        let mut output = OutputConfig::default();
        output.directory = PathBuf::from("./tmp");
        output.name = String::from("sink_test");

        let mut manager = OutputManager::new(output).unwrap();
        let mut records = VecRecordStream::new(vec![Record::Json(JsonRecord::new(
            test.as_object().unwrap().clone(),
        ))]);
        manager
            .write_artifact("test", String::from("test"), &mut records)
            .unwrap();
    }

    #[test]
    fn test_sink_local_create_log_file_and_report() {
        let test = json!({"test":"value"});
        let mut output = OutputConfig::default();
        output.format = OutputFormat::Csv;
        output.directory = PathBuf::from("./tmp");
        output.name = String::from("sink_test");

        let mut manager = OutputManager::new(output).unwrap();
        let mut records = VecRecordStream::new(vec![Record::Json(JsonRecord::new(
            test.as_object().unwrap().clone(),
        ))]);
        manager
            .write_artifact("test", String::from("test"), &mut records)
            .unwrap();
        error!("hello");
        manager.finalize().unwrap();
    }
}
