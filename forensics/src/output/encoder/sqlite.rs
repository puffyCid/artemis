use crate::output::{
    context::ArtifactContext,
    encoder::{
        artifact_encoder::{
            EncoderStreamWriter, StreamArtifactEncoder, StreamTarget, StreamWriter,
        },
        helper::{
            record::{extra_json, read_json_rows, value_as_i64, value_as_string},
            schema::{ColumnKind, InferredSchema, quote_identifier, unique_table_name},
        },
    },
    error::{OutputError, OutputResult},
    record::RecordStream,
};
use rusqlite::{Connection, Error, params_from_iter, types::Value as SqlValue};
use serde_json::{Map, Value};
use std::{
    collections::HashMap,
    fmt::{self, Formatter},
    path::Path,
};

/// Encodes artifact records into a single sqlite file
#[derive(Debug, PartialEq)]
pub(crate) struct SqliteEncoder;

impl StreamArtifactEncoder for SqliteEncoder {
    fn extension(&self) -> &str {
        "sqlite"
    }

    fn mime_type(&self) -> &str {
        // https://mimetype.io/application/vnd.sqlite3
        "application/vnd.sqlite3"
    }

    fn encode_stream(
        &self,
        target: StreamTarget,
        records: &mut dyn RecordStream,
        context: &ArtifactContext,
    ) -> OutputResult<EncoderStreamWriter> {
        // Open a persistence connection to the sqlite database during the entire collection
        // Should allow for faster transactions vs constantly opening the sqlite file
        let conn = open_connection(&target)?;

        let mut writer = SqliteWriter {
            target,
            conn,
            artifact_tables: HashMap::new(),
            tables: HashMap::new(),
        };

        let rows = read_json_rows(records, context, self.extension())?;
        let record_count = if rows.is_empty() {
            0
        } else {
            writer.insert_artifact_rows(&context.artifact_name, &rows)?
        };

        Ok(EncoderStreamWriter {
            writer: StreamWriter::Sqlite(writer),
            record_count,
        })
    }
}

/// Open the sqlite file for writing
fn open_connection(target: &StreamTarget) -> OutputResult<Connection> {
    let conn =
        Connection::open(&target.path).map_err(|err| sqlite_path_error(&target.path, err))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(sqlite_error)?;

    Ok(conn)
}

/// Convert a path-specific sqlite open error
fn sqlite_path_error(path: impl AsRef<Path>, err: Error) -> OutputError {
    OutputError::Encode(format!(
        "failed to open sqlite file {}: {err}",
        path.as_ref().display()
    ))
}

/// Active sqlite writer for artifact collection output
pub(crate) struct SqliteWriter {
    /// Full path to the streamed output file
    target: StreamTarget,
    /// Sqlite connection reused for the entire collection
    conn: Connection,
    /// Mapping of artifact name to created table name
    artifact_tables: HashMap<String, String>,
    /// Schema inferred for each created table
    tables: HashMap<String, InferredSchema>,
}

impl fmt::Debug for SqliteWriter {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SqliteWriter")
            .field("target", &self.target)
            .field("tables", &self.artifact_tables)
            .finish()
    }
}

impl SqliteWriter {
    /// Write the `RecordStream` into sqlite file
    pub(crate) fn write_records(
        &mut self,
        records: &mut dyn RecordStream,
        context: &ArtifactContext,
    ) -> OutputResult<usize> {
        let rows = read_json_rows(records, context, "sqlite")?;
        if rows.is_empty() {
            return Ok(0);
        }

        self.insert_artifact_rows(&context.artifact_name, &rows)
    }

    /// Complete the sqlite transaction and finalize the write output
    pub(crate) fn finish(self) -> OutputResult<()> {
        self.conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); PRAGMA journal_mode = DELETE;")
            .map_err(|err| {
                OutputError::Encode(format!(
                    "failed to close sqlite file {}: {err:?}",
                    self.target.path.display()
                ))
            })?;

        Ok(())
    }

    /// Insert the array of JSON artifacts into sqlite
    fn insert_artifact_rows(
        &mut self,
        artifact_name: &str,
        rows: &[Map<String, Value>],
    ) -> OutputResult<usize> {
        let table = self.ensure_table(artifact_name, rows)?;
        let schema = self.tables.get(&table).cloned().ok_or_else(|| {
            OutputError::Encode(format!("missing sqlite schema for table {table}"))
        })?;

        let columns = schema
            .columns
            .iter()
            .map(|column| quote_identifier(&column.column_name))
            .collect::<Vec<_>>()
            .join(", ");

        let placeholder = (1..=schema.columns.len())
            .map(|index| format!("?{index}"))
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!(
            "INSERT INTO {} ({columns}) VALUES ({placeholder})",
            quote_identifier(&table)
        );

        // Start inserting JSON records into sqlite file
        let transaction = self.conn.transaction().map_err(sqlite_error)?;
        {
            let mut statement = transaction.prepare(&sql).map_err(sqlite_error)?;
            for row in rows {
                let values = bind_values(&schema, row);
                statement
                    .execute(params_from_iter(values))
                    .map_err(sqlite_error)?;
            }
        }

        transaction.commit().map_err(sqlite_error)?;

        Ok(rows.len())
    }

    /// Validate that the sqlite table exists or create it
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
                    column.kind.sql_type()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        let sql = format!("CREATE TABLE {} ({definitions})", quote_identifier(&table));
        self.conn.execute(&sql, []).map_err(sqlite_error)?;

        self.artifact_tables
            .insert(artifact_name.to_string(), table.clone());
        self.tables.insert(table.clone(), schema);

        Ok(table)
    }
}

impl ColumnKind {
    /// Return the column type
    fn sql_type(self) -> &'static str {
        match self {
            Self::Bool | Self::Int64 => "INTEGER",
            Self::Double => "REAL",
            Self::Utf8 | Self::Json | Self::Timestamp | Self::UnsignedInt64 => "TEXT",
        }
    }
}

/// Convert the JSON data into supported array of sql data
fn bind_values(schema: &InferredSchema, row: &Map<String, Value>) -> Vec<SqlValue> {
    schema
        .columns
        .iter()
        .map(|column| match &column.source_name {
            Some(name) => value_for_column(column.kind, row.get(name)),
            None => extra_json(&schema.known_fields, row).map_or(SqlValue::Null, SqlValue::Text),
        })
        .collect()
}

/// Based on the column schema convert our JSON value to a compatible sql type
fn value_for_column(kind: ColumnKind, value: Option<&Value>) -> SqlValue {
    let Some(json_value) = value else {
        return SqlValue::Null;
    };

    match kind {
        ColumnKind::Bool => json_value.as_bool().map_or(SqlValue::Null, |flag| {
            SqlValue::Integer(if flag { 1 } else { 0 })
        }),
        ColumnKind::Int64 => value_as_i64(json_value).map_or(SqlValue::Null, SqlValue::Integer),
        ColumnKind::Double => json_value.as_f64().map_or(SqlValue::Null, SqlValue::Real),
        ColumnKind::Utf8 | ColumnKind::Json | ColumnKind::Timestamp | ColumnKind::UnsignedInt64 => {
            value_as_string(json_value).map_or(SqlValue::Null, SqlValue::Text)
        }
    }
}

/// Convert `rusqlite::Error` to `OutputError`
fn sqlite_error(err: Error) -> OutputError {
    OutputError::Encode(format!("sqlite error: {err}"))
}

#[cfg(test)]
mod tests {
    use super::SqliteEncoder;
    use crate::{
        output::{
            context::CollectionContext,
            encoder::artifact_encoder::{StreamArtifactEncoder, StreamTarget},
            record::{JsonRecord, Record, ScalarRecord, VecRecordStream},
        },
        structs::toml::OutputConfig,
    };
    use rusqlite::Connection;
    use serde_json::{Value, json};
    use std::{
        fs::{create_dir_all, remove_file},
        path::PathBuf,
    };

    fn test_context(artifact: &str) -> crate::output::context::ArtifactContext {
        let output = OutputConfig::default();
        CollectionContext::new(&output, PathBuf::from("./tmp/sqlite_test.log")).artifact(
            artifact,
            &output.start_time_filter,
            &output.end_time_filter,
        )
    }

    fn target(name: &str) -> StreamTarget {
        let path = PathBuf::from("./tmp").join(format!("{name}.sqlite"));
        let _ = create_dir_all("./tmp");
        let _ = remove_file(&path);
        let _ = remove_file(format!("{}.wal", path.display()));
        let _ = remove_file(format!("{}-wal", path.display()));
        let _ = remove_file(format!("{}-shm", path.display()));
        StreamTarget::new(path)
    }

    fn json_record(value: Value) -> Record {
        Record::Json(JsonRecord::new(value.as_object().unwrap().clone()))
    }

    fn table_names(path: &PathBuf) -> Vec<String> {
        let conn = Connection::open(path).unwrap();
        let mut statement = conn
            .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
            .unwrap();
        statement
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    fn column_names(path: &PathBuf, table: &str) -> Vec<String> {
        let conn = Connection::open(path).unwrap();
        let mut statement = conn
            .prepare(&format!("PRAGMA table_info(\"{table}\")"))
            .unwrap();
        statement
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .collect::<Result<Vec<_>, _>>()
            .unwrap()
    }

    fn table_count(path: &PathBuf, table: &str) -> i64 {
        let conn = Connection::open(path).unwrap();
        conn.query_row(&format!("SELECT COUNT(*) FROM \"{table}\""), [], |row| {
            row.get(0)
        })
        .unwrap()
    }

    #[test]
    fn test_sqlite_encode_stream() {
        let path = PathBuf::from("./tmp/sqlite_encode_stream.sqlite");
        let target = target("sqlite_encode_stream");
        let context = test_context("files");
        let encoder = SqliteEncoder;
        let mut records = VecRecordStream::new(vec![
            json_record(json!({"path": "/tmp/one", "size": 1})),
            json_record(json!({"path": "/tmp/two", "size": 2})),
        ]);
        let opened = encoder
            .encode_stream(target, &mut records, &context)
            .unwrap();
        assert_eq!(opened.record_count, 2);

        opened.writer.finish().unwrap();
        assert_eq!(table_names(&path), vec!["files"]);
        assert_eq!(table_count(&path, "files"), 2);

        let names = column_names(&path, "files");
        assert!(names.iter().any(|name| name == "path"));
        assert!(names.iter().any(|name| name == "size"));
        assert!(names.iter().any(|name| name == "collection_metadata"));
        assert!(names.iter().any(|name| name == "_extra_json"));
    }

    #[test]
    fn test_sqlite_write_records_second_chunk() {
        let path = PathBuf::from("./tmp/sqlite_second_chunk.sqlite");
        let target = target("sqlite_second_chunk");
        let context = test_context("files");
        let encoder = SqliteEncoder;
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
        assert_eq!(table_names(&path), vec!["files"]);
        assert_eq!(table_count(&path, "files"), 2);
    }

    #[test]
    fn test_sqlite_empty_first_chunk_has_no_table() {
        let path = PathBuf::from("./tmp/sqlite_empty_first_chunk.sqlite");
        let target = target("sqlite_empty_first_chunk");
        let context = test_context("files");
        let encoder = SqliteEncoder;
        let mut records = VecRecordStream::new(Vec::new());
        let opened = encoder
            .encode_stream(target, &mut records, &context)
            .unwrap();

        assert_eq!(opened.record_count, 0);
        opened.writer.finish().unwrap();
        assert!(table_names(&path).is_empty());
    }

    #[test]
    fn test_sqlite_empty_later_chunk_ok() {
        let path = PathBuf::from("./tmp/sqlite_empty_later_chunk.sqlite");
        let target = target("sqlite_empty_later_chunk");
        let context = test_context("files");
        let encoder = SqliteEncoder;
        let mut first = VecRecordStream::new(vec![json_record(json!({
            "path": "/tmp/one",
            "size": 1
        }))]);

        let mut opened = encoder.encode_stream(target, &mut first, &context).unwrap();
        let mut empty = VecRecordStream::new(Vec::new());
        let count = opened.writer.write_records(&mut empty, &context).unwrap();
        assert_eq!(count, 0);

        opened.writer.finish().unwrap();
        assert_eq!(table_names(&path), vec!["files"]);
        assert_eq!(table_count(&path, "files"), 1);
    }

    #[test]
    fn test_sqlite_unsupported_record() {
        let target = target("sqlite_unsupported_record");
        let context = test_context("files");
        let encoder = SqliteEncoder;
        let mut records = VecRecordStream::new(vec![Record::Scalar(ScalarRecord::Text(
            String::from("not json"),
        ))]);

        let err = encoder
            .encode_stream(target, &mut records, &context)
            .unwrap_err();
        assert!(err.to_string().contains("sqlite"));
        assert!(err.to_string().contains("text"));
    }

    #[test]
    fn test_sqlite_late_fields_extra_json_schema() {
        let path = PathBuf::from("./tmp/sqlite_late_fields.sqlite");
        let target = target("sqlite_late_fields");
        let context = test_context("files");
        let encoder = SqliteEncoder;
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
        let names = column_names(&path, "files");

        assert!(names.iter().any(|name| name == "path"));
        assert!(names.iter().any(|name| name == "_extra_json"));
        assert!(!names.iter().any(|name| name == "late_field"));
        assert_eq!(table_count(&path, "files"), 2);

        let conn = Connection::open(&path).unwrap();
        let extra: Option<String> = conn
            .query_row(
                "SELECT _extra_json FROM files WHERE path = '/tmp/two'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert!(extra.unwrap().contains("late_field"));
    }

    #[test]
    fn test_sqlite_two_artifacts_two_tables() {
        let path = PathBuf::from("./tmp/sqlite_two_artifacts.sqlite");
        let target = target("sqlite_two_artifacts");
        let files_context = test_context("files");
        let encoder = SqliteEncoder;
        let mut first = VecRecordStream::new(vec![json_record(json!({
            "path": "/tmp/one"
        }))]);

        let mut opened = encoder
            .encode_stream(target, &mut first, &files_context)
            .unwrap();
        let registry_context = test_context("registry");
        let mut second = VecRecordStream::new(vec![json_record(json!({
            "key": "Software"
        }))]);

        opened
            .writer
            .write_records(&mut second, &registry_context)
            .unwrap();
        opened.writer.finish().unwrap();

        assert_eq!(table_names(&path), vec!["files", "registry"]);
        assert_eq!(table_count(&path, "files"), 1);
        assert_eq!(table_count(&path, "registry"), 1);
    }
}
