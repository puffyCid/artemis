use crate::output::{
    context::ArtifactContext,
    encoder::{
        artifact_encoder::{
            EncoderStreamWriter, StreamArtifactEncoder, StreamTarget, StreamWriter,
        },
        metadata::append_metadata,
    },
    error::{OutputError, OutputResult},
    record::{Record, RecordStream},
};
use parquet::{
    basic::Compression,
    column::writer::ColumnWriter,
    data_type::ByteArray,
    errors::ParquetError,
    file::{
        properties::{EnabledStatistics, WriterProperties},
        writer::{SerializedColumnWriter, SerializedFileWriter},
    },
    schema::parser::parse_message_type,
};
use serde_json::{Map, Value};
use std::{
    collections::{HashMap, HashSet},
    fs::File,
    sync::Arc,
};

/// Encodes artifact records into a single Parquet file
#[derive(Debug, PartialEq)]
pub(crate) struct ParquetEncoder;

impl StreamArtifactEncoder for ParquetEncoder {
    fn extension(&self) -> &str {
        "parquet"
    }

    fn mime_type(&self) -> &str {
        "application/vnd.apache.parquet"
    }

    fn encode_stream(
        &self,
        target: StreamTarget,
        records: &mut dyn RecordStream,
        context: &ArtifactContext,
    ) -> OutputResult<EncoderStreamWriter> {
        // Convert first record chunk into parquet rows and append collection metadata
        let rows = read_json_rows(records, context)?;

        // Infer the parquet schema from the first non-empty record chunk
        let schema = ParquetSchema::infer(&rows);
        let message_type = schema.message_type();

        let parquet_schema = Arc::new(parse_message_type(&message_type).map_err(parquet_error)?);
        let props = Arc::new(
            WriterProperties::builder()
                .set_compression(Compression::SNAPPY)
                .set_statistics_enabled(EnabledStatistics::None)
                .build(),
        );

        // Open the parquet file writer using the inferred schema
        let file =
            File::create(&target.path).map_err(|err| OutputError::io_path(&target.path, err))?;
        let writer =
            SerializedFileWriter::new(file, parquet_schema, props).map_err(parquet_error)?;
        let mut parquet_writer = ParquetWriter {
            target,
            schema,
            writer,
        };

        // Write the first chunk as the first parquet row group
        parquet_writer.write_row_group(&rows)?;

        Ok(EncoderStreamWriter {
            writer: StreamWriter::Parquet(parquet_writer),
            record_count: rows.len(),
        })
    }
}

/// Converts JSON record values into parquet rows.
///
/// Parquet output currently supports only JSON object records.
fn read_json_rows(
    records: &mut dyn RecordStream,
    context: &ArtifactContext,
) -> OutputResult<Vec<Map<String, Value>>> {
    let mut rows = Vec::new();

    while let Some(record) = records.next_record()? {
        let Record::Json(record) = record else {
            return Err(OutputError::UnsupportedRecord {
                format: String::from("parquet"),
                record_type: record.kind().to_string(),
            });
        };

        let mut value = record.into_value();
        append_metadata(&mut value, context);
        let Value::Object(fields) = value else {
            return Err(OutputError::Encode(String::from(
                "parquet records must be JSON objects",
            )));
        };

        rows.push(fields);
    }

    Ok(rows)
}

/// Active parquet writer for one streamed artifact output
#[derive(Debug)]
pub(crate) struct ParquetWriter {
    /// Full path to the streamed output file
    target: StreamTarget,
    /// The parquet schema
    schema: ParquetSchema,
    /// Writer to the parquet file
    writer: SerializedFileWriter<File>,
}

impl ParquetWriter {
    /// Writes the next record chunk as a parquet row group
    pub(crate) fn write_records(
        &mut self,
        records: &mut dyn RecordStream,
        context: &ArtifactContext,
    ) -> OutputResult<usize> {
        let rows = read_json_rows(records, context)?;
        if rows.is_empty() {
            return Ok(0);
        }

        self.write_row_group(&rows)?;

        Ok(rows.len())
    }

    /// Finalizes the parquet file
    pub(crate) fn finish(self) -> OutputResult<()> {
        self.writer.close().map_err(|err| {
            OutputError::Encode(format!(
                "failed to close parquet file {}: {err}",
                self.target.path.display()
            ))
        })?;

        Ok(())
    }

    /// Writes rows to the parquet file as one row group
    fn write_row_group(&mut self, rows: &[Map<String, Value>]) -> OutputResult<()> {
        let columns = build_columns(&self.schema, rows);

        let mut row_group = self.writer.next_row_group().map_err(parquet_error)?;
        for column in columns {
            let Some(mut column_writer) = row_group.next_column().map_err(parquet_error)? else {
                return Err(OutputError::Encode(String::from(
                    "parquet schema expected more columns than writer returned",
                )));
            };

            write_column(&mut column_writer, column)?;
            column_writer.close().map_err(parquet_error)?;
        }

        row_group.close().map_err(parquet_error)?;

        Ok(())
    }
}

/// Schema inferred for a streamed parquet artifact
#[derive(Debug)]
struct ParquetSchema {
    /// Ordered parquet columns
    columns: Vec<ColumnSpec>,
    /// Source field names included in the inferred schema
    known_fields: HashSet<String>,
}

impl ParquetSchema {
    /// Infers a parquet schema from the first chunk of artifact rows
    fn infer(rows: &[Map<String, Value>]) -> Self {
        let mut order = Vec::new();
        let mut kinds: HashMap<String, ColumnKind> = HashMap::new();
        for row in rows {
            for (key, value) in row {
                if !kinds.contains_key(key) {
                    order.push(key.clone());
                    kinds.insert(key.clone(), ColumnKind::from_value(value));
                    continue;
                }

                let current = kinds.get(key).copied().unwrap_or(ColumnKind::Utf8);
                kinds.insert(key.clone(), current.merge(ColumnKind::from_value(value)));
            }
        }

        let mut used_names = HashSet::new();
        let mut known_fields = HashSet::new();
        let mut columns = Vec::new();

        for source_name in order {
            known_fields.insert(source_name.clone());

            let parquet_name = Self::unique_field_name(&source_name, &mut used_names);
            let kind = kinds.get(&source_name).copied().unwrap_or(ColumnKind::Utf8);

            columns.push(ColumnSpec {
                source_name: Some(source_name),
                parquet_name,
                kind,
            });
        }

        columns.push(ColumnSpec {
            source_name: None,
            parquet_name: Self::unique_field_name("_extra_json", &mut used_names),
            kind: ColumnKind::Utf8,
        });

        Self {
            columns,
            known_fields,
        }
    }

    /// Builds the parquet message type string
    fn message_type(&self) -> String {
        let mut message = String::from("message artemis {\n");
        for column in &self.columns {
            let field = match column.kind {
                ColumnKind::Bool => format!("  optional BOOLEAN {};\n", column.parquet_name),
                ColumnKind::Double => format!("  optional DOUBLE {};\n", column.parquet_name),
                ColumnKind::Utf8 => {
                    format!("  optional BYTE_ARRAY {} (UTF8);\n", column.parquet_name)
                }
                ColumnKind::Int64 => format!("  optional INT64 {};\n", column.parquet_name),
            };

            message.push_str(&field);
        }

        message.push_str("}\n");
        message
    }

    /// Converts a source field name into a unique parquet column name
    fn unique_field_name(source: &str, used: &mut HashSet<String>) -> String {
        let base = Self::sanitize_field_name(source);
        let mut candidate = base.clone();

        let mut suffix = 1;
        while used.contains(&candidate) {
            candidate = format!("{base}_{suffix}");
            suffix += 1;
        }

        used.insert(candidate.clone());
        candidate
    }

    /// Replaces unsupported parquet column-name characters with underscores
    fn sanitize_field_name(source: &str) -> String {
        let mut name = source
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || c == '_' {
                    c
                } else {
                    '_'
                }
            })
            .collect::<String>();

        if name.is_empty() {
            name = String::from("field");
        }

        if name.chars().next().is_some_and(|c| c.is_ascii_digit()) {
            name.insert(0, '_');
        }

        name
    }
}

/// Metadata for one parquet column
#[derive(Debug)]
struct ColumnSpec {
    /// Source JSON field name for the column
    source_name: Option<String>,
    /// The unique parquet column name
    parquet_name: String,
    /// Column type
    kind: ColumnKind,
}

/// Supported parquet column value types
#[derive(Copy, Clone, Debug)]
enum ColumnKind {
    Bool,
    Int64,
    Double,
    Utf8,
}

impl ColumnKind {
    /// Convert JSON value to `ColumnKind`
    fn from_value(value: &Value) -> Self {
        match value {
            Value::Bool(_) => Self::Bool,
            Value::Number(number) => {
                if number.is_i64() || number.as_u64().is_some_and(|n| i64::try_from(n).is_ok()) {
                    Self::Int64
                } else if number.is_f64() {
                    Self::Double
                } else {
                    Self::Utf8
                }
            }
            Value::Null | Value::Array(_) | Value::Object(_) | Value::String(_) => Self::Utf8,
        }
    }

    /// Merges inferred column types when a field has mixed value types in the schema chunk
    ///
    /// Example: `{"value": 1}` and later `{"value": 2.5}`
    ///
    /// The column type becomes `ColumnKind::Double`
    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Utf8, _) | (_, Self::Utf8) => Self::Utf8,
            (Self::Double, _) | (_, Self::Double) => Self::Double,
            (Self::Int64, Self::Int64) => Self::Int64,
            (Self::Bool, Self::Bool) => Self::Bool,
            _ => Self::Utf8,
        }
    }
}

/// Column values and definition levels prepared for parquet writing
enum ColumnBatch {
    Bool {
        /// Array of booleans
        values: Vec<bool>,
        /// Per-row definition levels; 1 means present, 0 means null or missing.
        definition_levels: Vec<i16>,
    },
    Int64 {
        /// Array of integers
        values: Vec<i64>,
        /// Per-row definition levels; 1 means present, 0 means null or missing.
        definition_levels: Vec<i16>,
    },
    Double {
        /// Array of floats
        values: Vec<f64>,
        /// Per-row definition levels; 1 means present, 0 means null or missing.
        definition_levels: Vec<i16>,
    },
    Utf8 {
        /// Array of `ByteArray`
        values: Vec<ByteArray>,
        /// Per-row definition levels; 1 means present, 0 means null or missing.
        definition_levels: Vec<i16>,
    },
}

/// Builds parquet column batches for the provided rows
fn build_columns(schema: &ParquetSchema, rows: &[Map<String, Value>]) -> Vec<ColumnBatch> {
    schema
        .columns
        .iter()
        .map(|column| build_column(schema, column, rows))
        .collect()
}

/// Assemble each column value
fn build_column(
    schema: &ParquetSchema,
    column: &ColumnSpec,
    rows: &[Map<String, Value>],
) -> ColumnBatch {
    match column.kind {
        ColumnKind::Bool => bool_column(column, rows),
        ColumnKind::Int64 => i64_column(column, rows),
        ColumnKind::Double => f64_column(column, rows),
        ColumnKind::Utf8 => utf8_column(schema, column, rows),
    }
}

/// Construct boolean column
fn bool_column(column: &ColumnSpec, rows: &[Map<String, Value>]) -> ColumnBatch {
    let mut values = Vec::new();
    let mut definition_levels = Vec::with_capacity(rows.len());
    for row in rows {
        let value = column
            .source_name
            .as_ref()
            .and_then(|name| row.get(name))
            .and_then(Value::as_bool);

        if let Some(val) = value {
            values.push(val);
            definition_levels.push(1);
            continue;
        }

        // Missing values will become null
        definition_levels.push(0);
    }

    ColumnBatch::Bool {
        values,
        definition_levels,
    }
}

/// Construct integer column
fn i64_column(column: &ColumnSpec, rows: &[Map<String, Value>]) -> ColumnBatch {
    let mut values = Vec::new();
    let mut definition_levels = Vec::with_capacity(rows.len());
    for row in rows {
        let value = column
            .source_name
            .as_ref()
            .and_then(|name| row.get(name))
            .and_then(value_as_i64);

        if let Some(val) = value {
            values.push(val);
            definition_levels.push(1);
            continue;
        }

        // Missing values will become null
        definition_levels.push(0);
    }

    ColumnBatch::Int64 {
        values,
        definition_levels,
    }
}

/// Attempt to convert JSON value to integer
fn value_as_i64(value: &Value) -> Option<i64> {
    let number = value.as_number()?;
    if let Some(val) = number.as_i64() {
        return Some(val);
    }

    let value = number.as_u64()?;
    i64::try_from(value).ok()
}

/// Construct float column
fn f64_column(column: &ColumnSpec, rows: &[Map<String, Value>]) -> ColumnBatch {
    let mut values = Vec::new();
    let mut definition_levels = Vec::with_capacity(rows.len());
    for row in rows {
        let value = column
            .source_name
            .as_ref()
            .and_then(|name| row.get(name))
            .and_then(Value::as_f64);

        if let Some(val) = value {
            values.push(val);
            definition_levels.push(1);
            continue;
        }

        // Missing values will become null
        definition_levels.push(0);
    }

    ColumnBatch::Double {
        values,
        definition_levels,
    }
}

/// Construct string column
fn utf8_column(
    schema: &ParquetSchema,
    column: &ColumnSpec,
    rows: &[Map<String, Value>],
) -> ColumnBatch {
    let mut values = Vec::new();
    let mut definition_levels = Vec::with_capacity(rows.len());
    for row in rows {
        let value = match &column.source_name {
            Some(name) => row.get(name).and_then(value_as_string),
            None => extra_json(schema, row),
        };

        if let Some(val) = value {
            values.push(ByteArray::from(val.as_str()));
            definition_levels.push(1);
            continue;
        }

        // Missing values will become null
        definition_levels.push(0);
    }

    ColumnBatch::Utf8 {
        values,
        definition_levels,
    }
}

/// Attempt to convert JSON value to string
fn value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(val) => Some(val.clone()),
        Value::Bool(val) => Some(val.to_string()),
        Value::Number(val) => Some(val.to_string()),
        Value::Array(val) => serde_json::to_string(val).ok(),
        Value::Object(val) => serde_json::to_string(val).ok(),
    }
}

/// Serializes fields not present in the inferred schema into the `_extra_json` column
fn extra_json(schema: &ParquetSchema, row: &Map<String, Value>) -> Option<String> {
    let mut extra = Map::new();
    for (key, value) in row {
        if !schema.known_fields.contains(key) {
            extra.insert(key.clone(), value.clone());
        }
    }

    if extra.is_empty() {
        return None;
    }

    serde_json::to_string(&Value::Object(extra)).ok()
}

/// Write the column to the parquet file
fn write_column(writer: &mut SerializedColumnWriter<'_>, column: ColumnBatch) -> OutputResult<()> {
    match (writer.untyped(), column) {
        (
            ColumnWriter::BoolColumnWriter(write),
            ColumnBatch::Bool {
                values,
                definition_levels,
            },
        ) => {
            write
                .write_batch(&values, Some(&definition_levels), None)
                .map_err(parquet_error)?;
        }
        (
            ColumnWriter::Int64ColumnWriter(write),
            ColumnBatch::Int64 {
                values,
                definition_levels,
            },
        ) => {
            write
                .write_batch(&values, Some(&definition_levels), None)
                .map_err(parquet_error)?;
        }
        (
            ColumnWriter::DoubleColumnWriter(write),
            ColumnBatch::Double {
                values,
                definition_levels,
            },
        ) => {
            write
                .write_batch(&values, Some(&definition_levels), None)
                .map_err(parquet_error)?;
        }
        (
            ColumnWriter::ByteArrayColumnWriter(write),
            ColumnBatch::Utf8 {
                values,
                definition_levels,
            },
        ) => {
            write
                .write_batch(&values, Some(&definition_levels), None)
                .map_err(parquet_error)?;
        }
        _ => {
            return Err(OutputError::Encode(String::from(
                "parquet column type did not match schema",
            )));
        }
    }

    Ok(())
}

/// Convert `ParquetError` to `OutputError`
fn parquet_error(err: ParquetError) -> OutputError {
    OutputError::Encode(format!("parquet error: {err}"))
}

#[cfg(test)]
mod tests {
    use super::ParquetEncoder;
    use crate::{
        output::{
            context::CollectionContext,
            encoder::artifact_encoder::StreamArtifactEncoder,
            encoder::artifact_encoder::StreamTarget,
            record::{JsonRecord, Record, ScalarRecord, VecRecordStream},
        },
        structs::toml::OutputConfig,
    };
    use parquet::file::reader::{FileReader, SerializedFileReader};
    use serde_json::{Value, json};
    use std::{fs::File, path::PathBuf};

    fn test_context() -> crate::output::context::ArtifactContext {
        let output = OutputConfig::default();

        CollectionContext::new(&output, PathBuf::from("./tmp/parquet_test.log")).artifact(
            "test",
            &output.start_time_filter,
            &output.end_time_filter,
        )
    }

    fn target(name: &str) -> StreamTarget {
        let path = PathBuf::from("./tmp").join(format!("{name}.parquet"));
        let _ = std::fs::create_dir_all("./tmp");
        let _ = std::fs::remove_file(&path);

        StreamTarget::new(path)
    }

    fn json_record(value: Value) -> Record {
        Record::Json(JsonRecord::new(value.as_object().unwrap().clone()))
    }

    fn parquet_metadata(path: &PathBuf) -> parquet::file::metadata::ParquetMetaData {
        let file = File::open(path).unwrap();
        let reader = SerializedFileReader::new(file).unwrap();
        reader.metadata().clone()
    }

    fn column_names(metadata: &parquet::file::metadata::ParquetMetaData) -> Vec<String> {
        metadata
            .file_metadata()
            .schema_descr()
            .columns()
            .iter()
            .map(|column| column.name().to_string())
            .collect()
    }

    #[test]
    fn test_parquet_encode_stream() {
        let path = PathBuf::from("./tmp/parquet_encode_stream.parquet");
        let target = target("parquet_encode_stream");
        let context = test_context();
        let encoder = ParquetEncoder;

        let mut records = VecRecordStream::new(vec![
            json_record(json!({"path": "/tmp/one", "size": 1})),
            json_record(json!({"path": "/tmp/two", "size": 2})),
        ]);

        let opened = encoder
            .encode_stream(target, &mut records, &context)
            .unwrap();

        assert_eq!(opened.record_count, 2);
        opened.writer.finish().unwrap();

        let metadata = parquet_metadata(&path);
        assert_eq!(metadata.file_metadata().num_rows(), 2);
        assert_eq!(metadata.num_row_groups(), 1);
    }

    #[test]
    fn test_parquet_write_records_second_chunk() {
        let path = PathBuf::from("./tmp/parquet_second_chunk.parquet");
        let target = target("parquet_second_chunk");
        let context = test_context();
        let encoder = ParquetEncoder;

        let mut first = VecRecordStream::new(vec![json_record(json!({
            "path": "/tmp/one",
            "size": 1
        }))]);

        let mut opened = encoder.encode_stream(target, &mut first, &context).unwrap();

        let mut second = VecRecordStream::new(vec![json_record(json!({
            "path": "/tmp/two",
            "size": 2
        }))]);

        let count = opened.writer.write_records(&mut second, &context).unwrap();
        assert_eq!(count, 1);

        opened.writer.finish().unwrap();

        let metadata = parquet_metadata(&path);
        assert_eq!(metadata.file_metadata().num_rows(), 2);
        assert_eq!(metadata.num_row_groups(), 2);
    }

    #[test]
    fn test_parquet_empty_first_chunk_error() {
        let target = target("parquet_empty_first_chunk");
        let context = test_context();
        let encoder = ParquetEncoder;
        let mut records = VecRecordStream::new(Vec::new());
        let opened = encoder
            .encode_stream(target, &mut records, &context)
            .unwrap();

        opened.writer.finish().unwrap();
        let path = PathBuf::from("./tmp/parquet_empty_first_chunk.parquet");

        let metadata = parquet_metadata(&path);
        let schema = metadata.file_metadata().schema_descr();
        let column = schema
            .columns()
            .iter()
            .find(|column| column.name() == "_extra_json")
            .unwrap();

        assert_eq!(column.physical_type(), parquet::basic::Type::BYTE_ARRAY);
    }

    #[test]
    fn test_parquet_empty_later_chunk_ok() {
        let path = PathBuf::from("./tmp/parquet_empty_later_chunk.parquet");
        let target = target("parquet_empty_later_chunk");
        let context = test_context();
        let encoder = ParquetEncoder;

        let mut first = VecRecordStream::new(vec![json_record(json!({
            "path": "/tmp/one",
            "size": 1
        }))]);

        let mut opened = encoder.encode_stream(target, &mut first, &context).unwrap();

        let mut empty = VecRecordStream::new(Vec::new());
        let count = opened.writer.write_records(&mut empty, &context).unwrap();
        assert_eq!(count, 0);

        opened.writer.finish().unwrap();

        let metadata = parquet_metadata(&path);
        assert_eq!(metadata.file_metadata().num_rows(), 1);
        assert_eq!(metadata.num_row_groups(), 1);
    }

    #[test]
    fn test_parquet_unsupported_record() {
        let target = target("parquet_unsupported_record");
        let context = test_context();
        let encoder = ParquetEncoder;

        let mut records = VecRecordStream::new(vec![Record::Scalar(ScalarRecord::Text(
            String::from("not json"),
        ))]);

        let err = encoder
            .encode_stream(target, &mut records, &context)
            .unwrap_err();

        assert!(err.to_string().contains("parquet"));
        assert!(err.to_string().contains("text"));
    }

    #[test]
    fn test_parquet_large_u64_is_utf8() {
        let path = PathBuf::from("./tmp/parquet_large_u64.parquet");
        let target = target("parquet_large_u64");
        let context = test_context();
        let encoder = ParquetEncoder;

        let mut records = VecRecordStream::new(vec![json_record(json!({
            "value": u64::MAX
        }))]);

        let opened = encoder
            .encode_stream(target, &mut records, &context)
            .unwrap();

        opened.writer.finish().unwrap();

        let metadata = parquet_metadata(&path);
        let schema = metadata.file_metadata().schema_descr();
        let column = schema
            .columns()
            .iter()
            .find(|column| column.name() == "value")
            .unwrap();

        assert_eq!(column.physical_type(), parquet::basic::Type::BYTE_ARRAY);
    }

    #[test]
    fn test_parquet_late_fields_extra_json_schema() {
        let path = PathBuf::from("./tmp/parquet_late_fields.parquet");
        let target = target("parquet_late_fields");
        let context = test_context();
        let encoder = ParquetEncoder;

        let mut first = VecRecordStream::new(vec![json_record(json!({
            "path": "/tmp/one"
        }))]);

        let mut opened = encoder.encode_stream(target, &mut first, &context).unwrap();

        let mut second = VecRecordStream::new(vec![json_record(json!({
            "path": "/tmp/two",
            "late_field": "value"
        }))]);

        opened.writer.write_records(&mut second, &context).unwrap();
        opened.writer.finish().unwrap();

        let metadata = parquet_metadata(&path);
        let names = column_names(&metadata);

        assert!(names.iter().any(|name| name == "path"));
        assert!(names.iter().any(|name| name == "_extra_json"));
        assert!(!names.iter().any(|name| name == "late_field"));
        assert_eq!(metadata.file_metadata().num_rows(), 2);
    }
}
