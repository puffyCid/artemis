use crate::output2::{
    error::OutputResult, report::CollectionReport, sink::output_handle::OutputHandle,
};
use std::{fs::File, io::Write, path::PathBuf};

pub(crate) struct LogOutput {
    pub(crate) path: PathBuf,
    pub(crate) file: File,
}

pub(crate) trait OutputSink {
    fn write_artifact(
        &mut self,
        artifact_name: &str,
        extension: &str,
        mime_type: &str,
        encode: &mut dyn FnMut(&mut dyn Write) -> OutputResult<usize>,
    ) -> OutputResult<OutputHandle>;

    fn write_report(&mut self, report: &CollectionReport) -> OutputResult<OutputHandle>;

    fn create_log_file(&mut self) -> OutputResult<LogOutput>;

    fn finalize(&mut self) -> OutputResult<()> {
        Ok(())
    }
}
