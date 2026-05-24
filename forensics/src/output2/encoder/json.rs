use crate::output2::{
    context::ArtifactContext,
    encoder::{artifact_encoder::ArtifactEncoder, metadata::append_metadata},
    error::OutputResult,
    record::{Record, RecordStream},
};
use std::io::Write;

/// Encoder for JSON files
#[derive(Debug, PartialEq)]
pub(crate) struct JsonEncoder;

impl ArtifactEncoder for JsonEncoder {
    fn mime_type(&self) -> &str {
        "application/json"
    }
    fn extension(&self) -> &str {
        "json"
    }
    fn encode(
        &self,
        records: &mut dyn RecordStream,
        writer: &mut dyn Write,
        context: &ArtifactContext,
    ) -> OutputResult<usize> {
        let mut count = 0;
        writer.write_all(b"[")?;

        while let Some(record) = records.next_record()? {
            let Record::Json(record) = record;
            if count > 0 {
                writer.write_all(b",")?;
            }

            let mut value = record.into_value();
            append_metadata(&mut value, context);
            serde_json::to_writer(&mut *writer, &value)?;

            count += 1;
        }
        writer.write_all(b"]")?;
        Ok(count)
    }
}
