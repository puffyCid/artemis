use crate::output2::{
    encoder::{artifact_encoder::ArtifactEncoder, metadata::append_metadata},
    error::OutputResult,
    record::Record,
};

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
        records: &mut dyn crate::output2::record::RecordStream,
        writer: &mut dyn std::io::Write,
        context: &crate::output2::context::ArtifactContext,
    ) -> OutputResult<usize> {
        let mut count = 0;

        while let Some(record) = records.next_record()? {
            let Record::Json(record) = record;
            let mut value = record.into_value();
            append_metadata(&mut value, context);
            serde_json::to_writer(&mut *writer, &value)?;

            count += 1;
        }

        Ok(count)
    }
}
