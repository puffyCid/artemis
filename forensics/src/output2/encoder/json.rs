use crate::output2::{
    context::ArtifactContext,
    encoder::{artifact_encoder::ArtifactEncoder, metadata::append_metadata},
    error::OutputResult,
    record::RecordStream,
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
            if count > 0 {
                writer.write_all(b",")?;
            }

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

            count += 1;
        }
        writer.write_all(b"]")?;
        Ok(count)
    }
}
