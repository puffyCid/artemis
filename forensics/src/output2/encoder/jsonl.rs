use crate::output2::{
    context::ArtifactContext,
    encoder::{artifact_encoder::ArtifactEncoder, metadata::append_metadata},
    error::OutputResult,
    record::RecordStream,
};
use std::io::Write;

/// Encoder for JSONL files
#[derive(Debug, PartialEq)]
pub(crate) struct JsonlEncoder;

impl ArtifactEncoder for JsonlEncoder {
    fn mime_type(&self) -> &str {
        "application/jsonl"
    }
    fn extension(&self) -> &str {
        "jsonl"
    }
    fn encode(
        &self,
        records: &mut dyn RecordStream,
        writer: &mut dyn Write,
        context: &ArtifactContext,
    ) -> OutputResult<usize> {
        let mut count = 0;

        while let Some(record) = records.next_record()? {
            let mut value = record.into_value()?;
            if let Some(value_array) = value.as_array_mut() {
                for value_record in value_array {
                    append_metadata(value_record, context);
                    serde_json::to_writer(&mut *writer, &value_record)?;
                    writer.write_all(b"\n")?;

                    count += 1;
                }
                continue;
            }
            append_metadata(&mut value, context);
            serde_json::to_writer(&mut *writer, &value)?;
            writer.write_all(b"\n")?;

            count += 1;
        }

        Ok(count)
    }
}
