use crate::output2::{
    config::{OutputConfig, OutputDestination},
    error::OutputResult,
    report::CollectionReport,
    sink::{
        local::LocalSink,
        output_handle::OutputHandle,
        output_sink::{LogOutput, OutputSink},
    },
};

#[cfg(feature = "api")]
use crate::output2::sink::api::ApiSink;
#[cfg(feature = "aws")]
use crate::output2::sink::aws::AwsSink;
#[cfg(feature = "azure")]
use crate::output2::sink::azure::AzureSink;
#[cfg(feature = "gcp")]
use crate::output2::sink::gcp::GcpSink;

/// Selected destination for encoded output.
///
/// `Sink` delegates destination writing to the configured output sink.
/// <https://en.wikipedia.org/wiki/Sink_(computing)>
///
/// All remote cloud bucket uploads try to follow a similar approach that Velociraptor uses
///
/// AWS - <https://docs.velociraptor.app/blog/2020/2020-07-14-triage-with-velociraptor-pt-4-cf0e60810d1e>
///
/// GCP - <https://docs.velociraptor.app/blog/2019/2019-10-08_triage-with-velociraptor-pt-3-d6f63215f579>
///
/// Azure - <https://docs.velociraptor.app/knowledge_base/tips/dropbox_server>
pub(crate) enum Sink {
    /// Write encoded output to local system
    Local(LocalSink),
    #[cfg(feature = "gcp")]
    Gcp(GcpSink),
    #[cfg(feature = "aws")]
    Aws(AwsSink),
    #[cfg(feature = "azure")]
    Azure(AzureSink),
    #[cfg(feature = "api")]
    Api(ApiSink),
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
            #[cfg(feature = "gcp")]
            Self::Gcp(sink) => sink.write_artifact(artifact_name, extension, mime_type, encode),
            #[cfg(feature = "aws")]
            Self::Aws(sink) => sink.write_artifact(artifact_name, extension, mime_type, encode),
            #[cfg(feature = "azure")]
            Self::Azure(sink) => sink.write_artifact(artifact_name, extension, mime_type, encode),
            #[cfg(feature = "api")]
            Self::Api(sink) => sink.write_artifact(artifact_name, extension, mime_type, encode),
        }
    }

    /// Finalizes destination-specific output.
    ///
    /// For local output, this may perform final zip compression and cleanup.
    pub(crate) fn finalize(&mut self) -> OutputResult<()> {
        match self {
            Self::Local(sink) => sink.finalize(),
            #[cfg(feature = "gcp")]
            Self::Gcp(sink) => sink.finalize(),
            #[cfg(feature = "aws")]
            Self::Aws(sink) => sink.finalize(),
            #[cfg(feature = "azure")]
            Self::Azure(sink) => sink.finalize(),
            #[cfg(feature = "api")]
            Self::Api(sink) => sink.finalize(),
        }
    }

    /// Initialize the log file
    pub(crate) fn create_log_file(&mut self) -> OutputResult<LogOutput> {
        match self {
            Self::Local(sink) => sink.create_log_file(),
            #[cfg(feature = "gcp")]
            Self::Gcp(sink) => sink.create_log_file(),
            #[cfg(feature = "aws")]
            Self::Aws(sink) => sink.create_log_file(),
            #[cfg(feature = "azure")]
            Self::Azure(sink) => sink.create_log_file(),
            #[cfg(feature = "api")]
            Self::Api(sink) => sink.create_log_file(),
        }
    }

    /// Once collection is completed, write a JSON collection report
    pub(crate) fn write_report(&mut self, report: &CollectionReport) -> OutputResult<OutputHandle> {
        match self {
            Self::Local(sink) => sink.write_report(report),
            #[cfg(feature = "gcp")]
            Self::Gcp(sink) => sink.write_report(report),
            #[cfg(feature = "aws")]
            Self::Aws(sink) => sink.write_report(report),
            #[cfg(feature = "azure")]
            Self::Azure(sink) => sink.write_report(report),
            #[cfg(feature = "api")]
            Self::Api(sink) => sink.write_report(report),
        }
    }
}

/// Setup a `Sink` to send data to selected output destination
pub(crate) fn build_sink(config: &OutputConfig) -> OutputResult<Sink> {
    match config.destination {
        OutputDestination::Local => Ok(Sink::Local(LocalSink::new(config)?)),
        #[cfg(feature = "gcp")]
        OutputDestination::Gcp => Ok(Sink::Gcp(GcpSink::new(config)?)),
        #[cfg(feature = "aws")]
        OutputDestination::Aws => Ok(Sink::Aws(AwsSink::new(config)?)),
        #[cfg(feature = "azure")]
        OutputDestination::Azure => Ok(Sink::Azure(AzureSink::new(config)?)),
        #[cfg(feature = "api")]
        OutputDestination::Api => Ok(Sink::Api(ApiSink::new(config)?)),
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
