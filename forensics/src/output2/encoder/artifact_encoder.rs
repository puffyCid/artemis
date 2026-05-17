use crate::output2::{
    context::ArtifactContext,
    encoder::{csv::CsvEncoder, json::JsonEncoder, jsonl::JsonlEncoder},
    error::OutputResult,
    record::RecordStream,
};
use std::io::Write;

pub(crate) enum Encoder {
    Json(JsonEncoder),
    Jsonl(JsonlEncoder),
    Csv(CsvEncoder),
}

impl Encoder {
    pub(crate) fn extension(&self) -> &str {
        match self {
            Self::Csv(encoder) => encoder.extension(),
            Self::Json(encoder) => encoder.extension(),
            Self::Jsonl(encoder) => encoder.extension(),
        }
    }

    pub(crate) fn mime_type(&self) -> &str {
        match self {
            Self::Csv(encoder) => encoder.mime_type(),
            Self::Json(encoder) => encoder.mime_type(),
            Self::Jsonl(encoder) => encoder.mime_type(),
        }
    }

    pub(crate) fn encode(
        &self,
        records: &mut dyn RecordStream,
        writer: &mut dyn Write,
        context: &ArtifactContext,
    ) -> OutputResult<usize> {
        match self {
            Self::Csv(encoder) => encoder.encode(records, writer, context),
            Self::Json(encoder) => encoder.encode(records, writer, context),
            Self::Jsonl(encoder) => encoder.encode(records, writer, context),
        }
    }
}

pub(crate) trait ArtifactEncoder {
    fn extension(&self) -> &str;
    fn mime_type(&self) -> &str;

    fn encode(
        &self,
        records: &mut dyn RecordStream,
        writer: &mut dyn Write,
        context: &ArtifactContext,
    ) -> OutputResult<usize>;
}
