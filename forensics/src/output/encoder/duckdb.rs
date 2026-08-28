use std::{
    collections::HashMap,
    fmt::{self, Formatter},
    path::Path,
};

use duckdb::{
    Connection, Error, appender_params_from_iter, params_from_iter,
    types::{TimeUnit, Value as DuckValue},
};
use serde_json::{Map, Value};

use crate::output::{
    context::ArtifactContext,
    encoder::{
        artifact_encoder::{EncoderStreamWriter, StreamArtifactEncoder, StreamTarget},
        helper::{
            record::{extra_json, read_json_rows, value_as_i64, value_as_string, value_as_u64},
            schema::{ColumnKind, InferredSchema, quote_identifier, unique_table_name},
        },
    },
    error::{OutputError, OutputResult},
    record::RecordStream,
};

/// Encodes artifact records into a single Duckdb file
#[derive(Debug, PartialEq)]
pub(crate) struct DuckEncoder;

impl StreamArtifactEncoder for DuckEncoder {
    fn extension(&self) -> &str {
        "duckdb"
    }

    fn mime_type(&self) -> &str {
        "application/vnd.duckdb"
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

fn open_connection(target: &StreamTarget) -> OutputResult<Connection> {
    let conn =
        Connection::open(&target.path).map_err(|err| duckdb_path_error(&target.path, err))?;

    Ok(conn)
}

/// Convert a path-specific duckdb open error
fn duckdb_path_error(path: impl AsRef<Path>, err: Error) -> OutputError {
    OutputError::Encode(format!(
        "failed to open duckdb file {}: {err}",
        path.as_ref().display()
    ))
}

/// Active duckdb writer for artifact collection output
pub(crate) struct DuckWriter {
    /// Full path to the streamed output file
    target: StreamTarget,
    /// Duckdb connection reused for the entire collection
    conn: Connection,
    /// Mapping of artifact name to created table name
    artifact_tables: HashMap<String, String>,
    /// Schema inferred for each created table
    tables: HashMap<String, InferredSchema>,
}

impl fmt::Debug for DuckWriter {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("DuckWriter")
            .field("target", &self.target)
            .field("tables", &self.artifact_tables)
            .finish()
    }
}

impl DuckWriter {
    pub(crate) fn write_records(
        &mut self,
        records: &mut dyn RecordStream,
        context: &ArtifactContext,
    ) -> OutputResult<usize> {
        let rows = read_json_rows(records, context, "duckdb")?;
        if rows.is_empty() {
            return Ok(0);
        }

        self.insert_artifact_rows(&context.artifact_name, &rows)
    }

    fn insert_artifact_rows(
        &mut self,
        artifact_name: &str,
        rows: &[Map<String, Value>],
    ) -> OutputResult<usize> {
        let table = self.ensure_table(artifact_name, rows)?;
        let schema = self.tables.get(&table).ok_or_else(|| {
            OutputError::Encode(format!("missing duckdb schema for table {table}"))
        })?;

        // Start inserting JSON records into duckdb file
        let transaction = self.conn.transaction().map_err(duckdb_error)?;
        {
            let mut appender = transaction.appender(&table).map_err(duckdb_error)?;
            for row in rows {
                appender
                    .append_row(appender_params_from_iter(bind_values(schema, row)))
                    .map_err(duckdb_error)?;
            }
            appender.flush().map_err(duckdb_error)?;
        }

        transaction.commit().map_err(duckdb_error)?;

        Ok(rows.len())
    }

    /// Validate that the duckdb table exists or create it
    fn ensure_table(
        &mut self,
        artifact_name: &str,
        rows: &[Map<String, Value>],
    ) -> OutputResult<String> {
        if let Some(table) = self.artifact_tables.get(artifact_name) {
            return Ok(table.clone());
        }

        let table = unique_table_name(artifact_name, &self.tables);

        // Create schema for a new table
        // Most columns will be TEXT
        let schema = InferredSchema::new(rows);

        let definitions = schema
            .columns
            .iter()
            .map(|column| {
                format!(
                    "{} {}",
                    quote_identifier(&column.column_name),
                    column.kind.duck_type()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!("CREATE TABLE {} ({definitions})", quote_identifier(&table));
        self.conn.execute(&sql, []).map_err(duckdb_error)?;

        self.artifact_tables
            .insert(artifact_name.to_string(), table.clone());
        self.tables.insert(table.clone(), schema);

        Ok(table)
    }
}

impl ColumnKind {
    /// Return the column type
    fn duck_type(self) -> &'static str {
        match self {
            Self::Int64 => "BIGINT",
            Self::Double => "DOUBLE",
            Self::Utf8 => "VARCHAR",
            Self::Bool => "BOOLEAN",
            Self::Json => "JSON",
            Self::Timestamp => "TIMESTAMP",
            Self::UnsignedInt64 => "UBIGINT",
        }
    }
}

/// Convert the JSON data into supported array of duck data
fn bind_values(schema: &InferredSchema, row: &Map<String, Value>) -> Vec<DuckValue> {
    schema
        .columns
        .iter()
        .map(|column| match &column.source_name {
            Some(name) => value_for_column(column.kind, row.get(name)),
            None => extra_json(&schema.known_fields, row).map_or(DuckValue::Null, DuckValue::Text),
        })
        .collect()
}

/// Based on the column schema convert our JSON value to a compatible duck type
fn value_for_column(kind: ColumnKind, value: Option<&Value>) -> DuckValue {
    let Some(json_value) = value else {
        return DuckValue::Null;
    };

    match kind {
        ColumnKind::Bool => json_value
            .as_bool()
            .map_or(DuckValue::Null, DuckValue::Boolean),
        ColumnKind::Int64 => value_as_i64(json_value).map_or(DuckValue::Null, DuckValue::BigInt),
        ColumnKind::Double => json_value
            .as_f64()
            .map_or(DuckValue::Null, DuckValue::Double),
        ColumnKind::Utf8 | ColumnKind::Json | ColumnKind::Timestamp => {
            value_as_string(json_value).map_or(DuckValue::Null, DuckValue::Text)
        }
        ColumnKind::UnsignedInt64 => {
            value_as_u64(json_value).map_or(DuckValue::Null, DuckValue::UBigInt)
        }
    }
}

/// Convert `duckdb_error::Error` to `OutputError`
fn duckdb_error(err: Error) -> OutputError {
    OutputError::Encode(format!("duckdb error: {err}"))
}
