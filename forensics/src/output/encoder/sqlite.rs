use crate::output::{
    context::ArtifactContext,
    encoder::artifact_encoder::{EncoderStreamWriter, StreamArtifactEncoder, StreamTarget},
    error::OutputResult,
    record::RecordStream,
};

/// Encodes artifact records into a single sqlite file
#[derive(Debug, PartialEq)]
pub(crate) struct SqliteEncoder;

impl StreamArtifactEncoder for SqliteEncoder {
    fn extension(&self) -> &str {
        "sqlite"
    }

    fn mime_type(&self) -> &str {
        "application/vnd.sqlite3"
    }

    fn encode_stream(
        &self,
        target: StreamTarget,
        records: &mut dyn RecordStream,
        context: &ArtifactContext,
    ) -> OutputResult<EncoderStreamWriter> {
        todo!()
    }
}
