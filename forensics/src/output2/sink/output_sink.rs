use crate::output2::{
    error::OutputResult, report::CollectionReport, sink::output_handle::OutputHandle,
};
use std::{fs::File, io::Write, path::PathBuf};

/// IO info associated with writing data to log file
pub(crate) struct LogOutput {
    /// Path to log file
    pub(crate) path: PathBuf,
    /// Log file target
    pub(crate) file: File,
}

/// Common interface for writing data to sink
pub(crate) trait OutputSink {
    /// Write artifact data to sink and return a small amount of data
    fn write_artifact(
        &mut self,
        artifact_name: &str,
        extension: &str,
        mime_type: &str,
        encode: &mut dyn FnMut(&mut dyn Write) -> OutputResult<usize>,
    ) -> OutputResult<OutputHandle>;

    /// Write report data to sink and return a small amount of data
    fn write_report(&mut self, report: &CollectionReport) -> OutputResult<OutputHandle>;

    /// Write log data to sink and return a small amount of data
    fn create_log_file(&mut self) -> OutputResult<LogOutput>;

    /// Complete writing data to the sink
    fn finalize(&mut self) -> OutputResult<()> {
        Ok(())
    }
}
