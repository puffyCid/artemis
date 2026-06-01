use crate::output2::{
    context::ArtifactContext,
    encoder::{
        csv::CsvEncoder, json::JsonEncoder, jsonl::JsonlEncoder, text::TextEncoder,
        timeline::TimelineEncoder,
    },
    error::OutputResult,
    record::RecordStream,
};
use std::io::Write;

/// A `Record` may be encoded into different formats
///
/// `Encoder` processes a `Record` entry into an output format
#[derive(Debug, PartialEq)]
pub(crate) enum Encoder {
    /// JSON array encoder
    Json(JsonEncoder),
    /// JSON Lines encoder
    Jsonl(JsonlEncoder),
    /// CSV encoder
    Csv(CsvEncoder),
    /// Timeline encoder
    Timeline(TimelineEncoder),
    /// Plaintext encoder
    Text(TextEncoder),
}

impl Encoder {
    /// Returns the extension for the output format
    pub(crate) fn extension(&self) -> &str {
        match self {
            Self::Csv(encoder) => encoder.extension(),
            Self::Json(encoder) => encoder.extension(),
            Self::Jsonl(encoder) => encoder.extension(),
            Self::Timeline(encoder) => encoder.extension(),
            Self::Text(encoder) => encoder.extension(),
        }
    }

    /// Returns ths MIME type for output format.
    ///
    /// Used for remote uploads
    pub(crate) fn mime_type(&self) -> &str {
        match self {
            Self::Csv(encoder) => encoder.mime_type(),
            Self::Json(encoder) => encoder.mime_type(),
            Self::Jsonl(encoder) => encoder.mime_type(),
            Self::Timeline(encoder) => encoder.mime_type(),
            Self::Text(encoder) => encoder.mime_type(),
        }
    }

    /// Encodes a `RecordStream` into select output format
    ///
    /// Returns number of records written
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
            Self::Timeline(encoder) => encoder.encode(records, writer, context),
            Self::Text(encoder) => encoder.encode(records, writer, context),
        }
    }
}

/// Common interface for artifact encoders
pub(crate) trait ArtifactEncoder {
    /// Returns the extension for the output format
    fn extension(&self) -> &str;
    /// Returns ths MIME type for output format.
    ///
    /// Used for remote uploads
    fn mime_type(&self) -> &str;
    /// Encodes a `RecordStream` into select output format
    ///
    /// Returns number of records written
    fn encode(
        &self,
        records: &mut dyn RecordStream,
        writer: &mut dyn Write,
        context: &ArtifactContext,
    ) -> OutputResult<usize>;
}

#[cfg(test)]
mod tests {
    use crate::output2::{
        config::OutputConfig,
        context::CollectionContext,
        encoder::{
            artifact_encoder::Encoder, csv::CsvEncoder, json::JsonEncoder, jsonl::JsonlEncoder,
            text::TextEncoder,
        },
        record::{JsonRecord, Record, ScalarRecord, VecRecordStream},
    };
    use serde_json::{Value, json};
    use std::{io::Cursor, path::PathBuf};

    #[test]
    fn test_encoder() {
        let test = json!({"test":"value"});
        let output = OutputConfig::default();
        let context = &CollectionContext::new(&output, PathBuf::from("./tmp")).artifact(
            "test",
            &output.start_time_filter,
            &output.end_time_filter,
        );

        let mut writer = Cursor::new(Vec::new());
        let csv_encoder = Encoder::Csv(CsvEncoder);
        let count = csv_encoder
            .encode(
                &mut VecRecordStream::new(vec![Record::Json(JsonRecord::new(
                    test.as_object().unwrap().clone(),
                ))]),
                &mut writer,
                context,
            )
            .unwrap();
        assert_eq!(csv_encoder.extension(), "csv");
        assert_eq!(csv_encoder.mime_type(), "text/csv");
        assert_eq!(count, 1);

        let json_encoder = Encoder::Json(JsonEncoder);
        let count = json_encoder
            .encode(
                &mut VecRecordStream::new(vec![Record::Json(JsonRecord::new(
                    test.as_object().unwrap().clone(),
                ))]),
                &mut writer,
                context,
            )
            .unwrap();
        assert_eq!(json_encoder.extension(), "json");
        assert_eq!(json_encoder.mime_type(), "application/json");
        assert_eq!(count, 1);

        let jsonl_encoder = Encoder::Jsonl(JsonlEncoder);
        let count = jsonl_encoder
            .encode(
                &mut VecRecordStream::new(vec![Record::Json(JsonRecord::new(
                    test.as_object().unwrap().clone(),
                ))]),
                &mut writer,
                context,
            )
            .unwrap();
        assert_eq!(jsonl_encoder.extension(), "jsonl");
        assert_eq!(jsonl_encoder.mime_type(), "application/jsonl");
        assert_eq!(count, 1);
    }

    #[test]
    fn test_text_encoder() {
        let text_encoder = Encoder::Text(TextEncoder);
        let output = OutputConfig::default();

        let context = &CollectionContext::new(&output, PathBuf::from("./tmp")).artifact(
            "test",
            &output.start_time_filter,
            &output.end_time_filter,
        );

        let mut writer = Cursor::new(Vec::new());
        let count = text_encoder
            .encode(
                &mut VecRecordStream::new(vec![Record::Scalar(
                    ScalarRecord::from_value(Value::String("test".into())).unwrap(),
                )]),
                &mut writer,
                context,
            )
            .unwrap();

        assert_eq!(text_encoder.extension(), "txt");
        assert_eq!(text_encoder.mime_type(), "text/plain");
        assert_eq!(count, 1);
    }
}
