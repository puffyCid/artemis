use crate::output::{
    context::ArtifactContext,
    encoder::{artifact_encoder::ArtifactEncoder, metadata::append_metadata},
    error::{OutputError, OutputResult},
    record::{Record, RecordStream},
};
use log::debug;
use std::io::Write;
use timeline::timeline::timeline_artifact;

/// Encoder for Timeline files. This is same as JSONL encoder except we do extra processing to timeline the data
#[derive(Debug, PartialEq)]
pub(crate) struct TimelineEncoder;

impl ArtifactEncoder for TimelineEncoder {
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
            let Record::Json(record) = record else {
                return Err(OutputError::unsupported_record("timeline", record.kind()));
            };
            let mut value = record.into_value();
            // If false skip writing
            if !timeline_artifact(
                &mut value,
                &context.artifact_name,
                &context.start_time_filter,
                &context.end_time_filter,
            ) {
                debug!(
                    "Skipping '{}' record during timeline encoding. Unexpected artifact format.",
                    context.artifact_name
                );
                continue;
            }
            if let Some(value_array) = value.as_array_mut() {
                for entry in value_array {
                    append_metadata(entry, context);
                    serde_json::to_writer(&mut *writer, entry)?;
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
