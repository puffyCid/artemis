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
        properties::WriterProperties,
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
        let rows = read_json_rows(records, context)?;

        if rows.is_empty() {
            return Err(OutputError::Encode(String::from(
                "cannot create parquet schema from empty data",
            )));
        }

        let schema = ParquetSchema::infer(&rows);
        let message_type = schema.message_type();

        let parquet_schema = Arc::new(parse_message_type(&message_type).map_err(parquet_error)?);
        let props = Arc::new(
            WriterProperties::builder()
                .set_compression(Compression::SNAPPY)
                .build(),
        );

        let file =
            File::create(&target.path).map_err(|err| OutputError::io_path(&target.path, err))?;
        let writer =
            SerializedFileWriter::new(file, parquet_schema, props).map_err(parquet_error)?;
        let mut parquet_writer = ParquetWriter {
            target,
            schema,
            writer,
        };

        parquet_writer.write_row_group(&rows)?;

        Ok(EncoderStreamWriter {
            writer: StreamWriter::Parquet(parquet_writer),
            record_count: rows.len(),
        })
    }
}

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

pub(crate) struct ParquetWriter {
    target: StreamTarget,
    schema: ParquetSchema,
    writer: SerializedFileWriter<File>,
}

impl ParquetWriter {
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

    pub(crate) fn finish(self) -> OutputResult<()> {
        self.writer.close().map_err(|err| {
            OutputError::Encode(format!(
                "failed to close parquet file {}: {err}",
                self.target.path.display()
            ))
        })?;

        Ok(())
    }

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

struct ParquetSchema {
    columns: Vec<ColumnSpec>,
    known_fields: HashSet<String>,
}

impl ParquetSchema {
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

struct ColumnSpec {
    source_name: Option<String>,
    parquet_name: String,
    kind: ColumnKind,
}

#[derive(Copy, Clone)]
enum ColumnKind {
    Bool,
    Int64,
    Double,
    Utf8,
}

impl ColumnKind {
    fn from_value(value: &Value) -> Self {
        match value {
            Value::Bool(_) => Self::Bool,
            Value::Number(number) => {
                if number.is_i64() || number.as_u64().is_some_and(|n| n <= i64::MAX as u64) {
                    Self::Int64
                } else {
                    Self::Double
                }
            }
            Value::Null => Self::Utf8,
            Value::Array(_) | Value::Object(_) | Value::String(_) => Self::Utf8,
        }
    }

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

enum ColumnBatch {
    Bool {
        values: Vec<bool>,
        def_levels: Vec<i16>,
    },
    Int64 {
        values: Vec<i64>,
        def_levels: Vec<i16>,
    },
    Double {
        values: Vec<f64>,
        def_levels: Vec<i16>,
    },
    Utf8 {
        values: Vec<ByteArray>,
        def_levels: Vec<i16>,
    },
}

fn build_columns(schema: &ParquetSchema, rows: &[Map<String, Value>]) -> Vec<ColumnBatch> {
    schema
        .columns
        .iter()
        .map(|column| build_column(schema, column, rows))
        .collect()
}

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

fn bool_column(column: &ColumnSpec, rows: &[Map<String, Value>]) -> ColumnBatch {
    let mut values = Vec::new();
    let mut def_levels = Vec::with_capacity(rows.len());
    for row in rows {
        let value = column
            .source_name
            .as_ref()
            .and_then(|name| row.get(name))
            .and_then(Value::as_bool);

        if let Some(val) = value {
            values.push(val);
            def_levels.push(1);
            continue;
        }

        def_levels.push(0);
    }

    ColumnBatch::Bool { values, def_levels }
}

fn i64_column(column: &ColumnSpec, rows: &[Map<String, Value>]) -> ColumnBatch {
    let mut values = Vec::new();
    let mut def_levels = Vec::with_capacity(rows.len());
    for row in rows {
        let value = column
            .source_name
            .as_ref()
            .and_then(|name| row.get(name))
            .and_then(value_as_i64);

        if let Some(val) = value {
            values.push(val);
            def_levels.push(1);
            continue;
        }

        def_levels.push(0);
    }

    ColumnBatch::Int64 { values, def_levels }
}

fn value_as_i64(value: &Value) -> Option<i64> {
    let number = value.as_number()?;
    if let Some(val) = number.as_i64() {
        return Some(val);
    }

    let value = number.as_u64()?;
    i64::try_from(value).ok()
}

fn f64_column(column: &ColumnSpec, rows: &[Map<String, Value>]) -> ColumnBatch {
    let mut values = Vec::new();
    let mut def_levels = Vec::with_capacity(rows.len());
    for row in rows {
        let value = column
            .source_name
            .as_ref()
            .and_then(|name| row.get(name))
            .and_then(Value::as_f64);

        if let Some(val) = value {
            values.push(val);
            def_levels.push(1);
            continue;
        }

        def_levels.push(0);
    }

    ColumnBatch::Double { values, def_levels }
}

fn utf8_column(
    schema: &ParquetSchema,
    column: &ColumnSpec,
    rows: &[Map<String, Value>],
) -> ColumnBatch {
    let mut values = Vec::new();
    let mut def_levels = Vec::with_capacity(rows.len());
    for row in rows {
        let value = match &column.source_name {
            Some(name) => row.get(name).and_then(value_as_string),
            None => extra_json(schema, row),
        };

        if let Some(val) = value {
            values.push(ByteArray::from(val.as_str()));
            def_levels.push(1);
            continue;
        }

        def_levels.push(0);
    }

    ColumnBatch::Utf8 { values, def_levels }
}

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

fn write_column(writer: &mut SerializedColumnWriter<'_>, column: ColumnBatch) -> OutputResult<()> {
    match (writer.untyped(), column) {
        (ColumnWriter::BoolColumnWriter(write), ColumnBatch::Bool { values, def_levels }) => {
            write
                .write_batch(&values, Some(&def_levels), None)
                .map_err(parquet_error)?;
        }
        (ColumnWriter::Int64ColumnWriter(write), ColumnBatch::Int64 { values, def_levels }) => {
            write
                .write_batch(&values, Some(&def_levels), None)
                .map_err(parquet_error)?;
        }
        (ColumnWriter::DoubleColumnWriter(write), ColumnBatch::Double { values, def_levels }) => {
            write
                .write_batch(&values, Some(&def_levels), None)
                .map_err(parquet_error)?;
        }
        (ColumnWriter::ByteArrayColumnWriter(write), ColumnBatch::Utf8 { values, def_levels }) => {
            write
                .write_batch(&values, Some(&def_levels), None)
                .map_err(parquet_error)?;
        }
        _ => {
            return Err(OutputError::Encode(String::from(
                "parquet column type did match schema",
            )));
        }
    }

    Ok(())
}

fn parquet_error(err: ParquetError) -> OutputError {
    OutputError::Encode(format!("parquet error: {err}"))
}
