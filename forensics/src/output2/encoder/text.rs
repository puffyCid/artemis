use crate::output2::{
    context::ArtifactContext,
    encoder::artifact_encoder::ArtifactEncoder,
    error::{OutputError, OutputResult},
    record::{Record, RecordStream},
};
use std::io::Write;

/// Encoder for plaintext files
#[derive(Debug, PartialEq)]
pub(crate) struct TextEncoder;

impl ArtifactEncoder for TextEncoder {
    fn extension(&self) -> &str {
        "txt"
    }

    fn mime_type(&self) -> &str {
        "text/plain"
    }

    fn encode(
        &self,
        records: &mut dyn RecordStream,
        writer: &mut dyn Write,
        _context: &ArtifactContext,
    ) -> OutputResult<usize> {
        let mut count = 0;
        while let Some(record) = records.next_record()? {
            match record {
                Record::Json(_) => {
                    return Err(OutputError::unsupported_record("txt", "json"));
                }
                Record::Scalar(value) => {
                    writer.write_all(value.to_text().as_bytes())?;
                    writer.write_all(b"\n")?;
                }
                Record::Array(_) => {
                    let value = record.into_value()?;
                    writer.write_all(value.to_string().as_bytes())?;
                    writer.write_all(b"\n")?;
                }
                Record::Null => writer.write_all(b"null\n")?,
            }
            count += 1;
        }
        Ok(count)
    }
}
