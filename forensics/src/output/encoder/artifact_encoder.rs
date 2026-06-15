use crate::output::{
    context::ArtifactContext,
    encoder::{
        csv::CsvEncoder,
        json::JsonEncoder,
        jsonl::JsonlEncoder,
        parquet::{ParquetEncoder, ParquetWriter},
        text::TextEncoder,
        timeline::TimelineEncoder,
        xml::XmlEncoder,
    },
    error::{OutputError, OutputResult},
    record::RecordStream,
};
use std::{io::Write, path::PathBuf};

/// Describes how the encoder will write artifact records
#[derive(Debug, PartialEq)]
pub(crate) enum EncoderMode {
    /// Artifact records are written in chunks. Each artifact records is written to a separate file
    ///
    /// For example, `EventLogs` are written in chunks to multiple JSONL files
    Chunked,
    /// Artifact records are streamed into a single file
    ///
    /// For example, `EventLogs` are streamed into a single Parquet file
    Streamed,
}

/// Target file for streamed output
#[derive(Debug)]
pub(crate) struct StreamTarget {
    /// Full path to the streamed output file
    pub(crate) path: PathBuf,
}

impl StreamTarget {
    /// Creates a new `StreamTarget` from a file path
    pub(crate) fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

/// Active writer for a streamed output format
#[derive(Debug)]
pub(crate) enum StreamWriter {
    /// Stream the output to a single parquet file on disk
    Parquet(ParquetWriter),
}

/// Writer returned after opening a streamed encoder
///
/// `record_count` is the number of records written while opening the stream
#[derive(Debug)]
pub(crate) struct EncoderStreamWriter {
    /// Writer that writes results to a file on disk
    pub(crate) writer: StreamWriter,
    /// Number of records written
    pub(crate) record_count: usize,
}

impl StreamWriter {
    /// Write `Record` values to disk using the `StreamWriter`
    ///
    /// Returns number of records written
    pub(crate) fn write_records(
        &mut self,
        records: &mut dyn RecordStream,
        context: &ArtifactContext,
    ) -> OutputResult<usize> {
        match self {
            Self::Parquet(writer) => writer.write_records(records, context),
        }
    }

    /// Finish streaming the `Record` values to disk
    pub(crate) fn finish(self) -> OutputResult<()> {
        match self {
            Self::Parquet(writer) => writer.finish(),
        }
    }
}

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
    /// XML encoder
    Xml(XmlEncoder),
    /// Parquet encoder
    Parquet(ParquetEncoder),
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
            Self::Xml(encoder) => encoder.extension(),
            Self::Parquet(encoder) => encoder.extension(),
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
            Self::Xml(encoder) => encoder.mime_type(),
            Self::Parquet(encoder) => encoder.mime_type(),
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
            Self::Xml(encoder) => encoder.encode(records, writer, context),
            Self::Parquet(_) => Err(OutputError::Encode(String::from(
                "parquet output is streamed; use 'encode_stream' instead",
            ))),
        }
    }

    /// Opens a streamed output writer and writes the first record chunk
    pub(crate) fn encode_stream(
        &self,
        target: StreamTarget,
        records: &mut dyn RecordStream,
        context: &ArtifactContext,
    ) -> OutputResult<EncoderStreamWriter> {
        match self {
            Self::Parquet(encoder) => encoder.encode_stream(target, records, context),
            _ => Err(OutputError::Encode(format!(
                "{} output is chunked and does not support streamed writers",
                self.extension()
            ))),
        }
    }

    /// Returns whether this encoder writes chunked files or a streamed file
    pub(crate) fn encoder_mode(&self) -> EncoderMode {
        match self {
            Encoder::Json(_)
            | Encoder::Text(_)
            | Encoder::Jsonl(_)
            | Encoder::Timeline(_)
            | Encoder::Csv(_)
            | Encoder::Xml(_) => EncoderMode::Chunked,
            Encoder::Parquet(_) => EncoderMode::Streamed,
        }
    }
}

/// Common interface for chunked artifact encoders
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

/// Common interface for streamed artifact encoders
pub(crate) trait StreamArtifactEncoder {
    /// Returns the extension for the output format
    fn extension(&self) -> &str;
    /// Returns ths MIME type for output format.
    ///
    /// Used for remote uploads
    fn mime_type(&self) -> &str;
    /// Encodes a `RecordStream` into select streaming output format
    ///
    /// Returns a writer to stream data to disk
    fn encode_stream(
        &self,
        target: StreamTarget,
        records: &mut dyn RecordStream,
        context: &ArtifactContext,
    ) -> OutputResult<EncoderStreamWriter>;
}

#[cfg(test)]
mod tests {
    use crate::{
        output::{
            context::CollectionContext,
            encoder::{
                artifact_encoder::{Encoder, StreamTarget},
                csv::CsvEncoder,
                json::JsonEncoder,
                jsonl::JsonlEncoder,
                parquet::ParquetEncoder,
                text::TextEncoder,
            },
            error::OutputError,
            record::{JsonRecord, Record, ScalarRecord, SingleRecordStream, VecRecordStream},
        },
        structs::toml::OutputConfig,
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
        assert_eq!(String::from_utf8(writer.into_inner()).unwrap(), "test\n");
    }

    #[test]
    fn test_text_encoder_array() {
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
                &mut VecRecordStream::new(vec![
                    Record::from_value(json!(["one", 2, true, 3.14])).unwrap(),
                ]),
                &mut writer,
                context,
            )
            .unwrap();

        assert_eq!(text_encoder.extension(), "txt");
        assert_eq!(text_encoder.mime_type(), "text/plain");
        assert_eq!(count, 1);

        let output = String::from_utf8(writer.into_inner()).unwrap();
        assert_eq!(output, "[\"one\",2,true,3.14]\n");
    }

    #[test]
    fn test_stream_encoder_parquet() {
        let test = json!({"test":"value"});
        let output = OutputConfig::default();
        let context = &CollectionContext::new(&output, PathBuf::from("./tmp")).artifact(
            "test",
            &output.start_time_filter,
            &output.end_time_filter,
        );
        let path = PathBuf::from("./tmp/parquet_encoder");
        let target = StreamTarget::new(path);
        let mut mem_writer = Cursor::new(Vec::new());

        let par_encoder = Encoder::Parquet(ParquetEncoder);
        let err = par_encoder
            .encode(
                &mut SingleRecordStream::new(Record::Json(JsonRecord::new(
                    test.as_object().unwrap().clone(),
                ))),
                &mut mem_writer,
                context,
            )
            .unwrap_err();
        assert!(
            matches!(err, OutputError::Encode(value) if value == "parquet output is streamed; use 'encode_stream' instead")
        );
        let writer = par_encoder
            .encode_stream(
                target,
                &mut SingleRecordStream::new(Record::Json(JsonRecord::new(
                    test.as_object().unwrap().clone(),
                ))),
                context,
            )
            .unwrap();

        assert_eq!(writer.record_count, 1);
        writer.writer.finish().unwrap();
    }
}
