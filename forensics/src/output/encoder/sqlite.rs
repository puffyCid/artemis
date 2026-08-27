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
use rusqlite::{Connection, params_from_iter, types::Value as SqlValue};
use serde_json::{Map, Value};
use std::{
    collections::{HashMap, HashSet},
    fmt::{self, Formatter},
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
        // Open a persistence connection to the sqlite database during the entire collection
        // Should allow for faster transactions vs constantly opening the sqlite file
        let conn = open_connection(&target)?;

        let mut writer = SqliteWriter {
            target,
            conn,
            artifact_tables: HashMap::new(),
            tables: HashMap::new(),
        };

        let rows = read_json_rows(records, context)?;
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

/// Convert the `RecordStream` artifact entry to a basic array of JSON
fn read_json_rows(
    records: &mut dyn RecordStream,
    context: &ArtifactContext,
) -> OutputResult<Vec<Map<String, Value>>> {
    let mut rows = Vec::new();

    // Loop through all artifact entries
    while let Some(record) = records.next_record()? {
        let Record::Json(json_record) = record else {
            return Err(OutputError::UnsupportedRecord {
                format: String::from("sqlite"),
                record_type: record.kind().to_string(),
            });
        };

        let mut value = json_record.into_value();
        append_metadata(&mut value, context);
        let Value::Object(fields) = value else {
            return Err(OutputError::Encode(String::from(
                "sqlite records must be JSON objects",
            )));
        };

        rows.push(fields);
    }

    Ok(rows)
}

/// Open the sqlite file for writing
fn open_connection(target: &StreamTarget) -> OutputResult<Connection> {
    let conn =
        Connection::open(&target.path).map_err(|err| sqlite_path_error(&target.path, err))?;
    conn.pragma_update(None, "journal_mode", "WAL")
        .map_err(sqlite_error)?;

    Ok(conn)
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
    tables: HashMap<String, SqliteSchema>,
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
        let rows = read_json_rows(records, context)?;
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
            .map(|column| quote_identifier(&column.sql_name))
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
        let schema = SqliteSchema::infer(rows);
        let definitions = schema
            .columns
            .iter()
            .map(|column| {
                format!(
                    "{} {}",
                    quote_identifier(&column.sql_name),
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

/// Schema associated with the artifact table
#[derive(Clone, Debug)]
struct SqliteSchema {
    /// Columns associated with the table
    columns: Vec<ColumnSpec>,
    /// Known columns inserted into table from first insertion
    known_fields: HashSet<String>,
}

impl SqliteSchema {
    /// Determine the JSON key types and try to create table schema
    fn infer(rows: &[Map<String, Value>]) -> Self {
        let mut order = Vec::new();
        let mut column_kinds = HashMap::new();

        // Loop through JSON data and determine value type
        for row in rows {
            for (key, value) in row {
                if !column_kinds.contains_key(key) {
                    order.push(key.clone());
                    column_kinds.insert(key.clone(), ColumnKind::from_value(value));
                    continue;
                }

                let current = column_kinds.get(key).copied().unwrap_or(ColumnKind::Text);
                column_kinds.insert(key.clone(), current.merge(ColumnKind::from_value(value)));
            }
        }

        let mut used_names = HashSet::new();
        let mut known_fields = HashSet::new();
        let mut columns = Vec::new();

        for source_name in order {
            known_fields.insert(source_name.clone());

            let sql_name = unique_field_name(&source_name, &mut used_names);
            let kind = column_kinds
                .get(&source_name)
                .copied()
                .unwrap_or(ColumnKind::Text);

            columns.push(ColumnSpec {
                source_name: Some(source_name),
                sql_name,
                kind,
            });
        }

        columns.push(ColumnSpec {
            source_name: None,
            sql_name: unique_field_name("_extra_json", &mut used_names),
            kind: ColumnKind::Text,
        });

        Self {
            columns,
            known_fields,
        }
    }
}

/// Metadata for one sqlite column
#[derive(Clone, Debug)]
struct ColumnSpec {
    /// Source JSON field name for the column
    source_name: Option<String>,
    /// The unique sqlite column name
    sql_name: String,
    /// Column type
    kind: ColumnKind,
}
/// Supported sqlite column value types
#[derive(Copy, Clone, Debug)]
enum ColumnKind {
    Bool,
    Int64,
    Double,
    Text,
}

impl ColumnKind {
    /// Determine Column type based on JSON value type
    fn from_value(value: &Value) -> Self {
        match value {
            Value::Bool(_) => Self::Bool,
            Value::Number(number) => {
                if number.is_i64() || number.as_u64().is_some_and(|n| i64::try_from(n).is_ok()) {
                    Self::Int64
                } else if number.is_f64() {
                    Self::Double
                } else {
                    Self::Text
                }
            }
            Value::Array(_) | Value::Null | Value::Object(_) | Value::String(_) => Self::Text,
        }
    }

    /// Merges inferred column types when a field has mixed value types in the schema chunk
    fn merge(self, other: Self) -> Self {
        match (self, other) {
            (Self::Text, _) | (_, Self::Text) => Self::Text,
            (Self::Double, _) | (_, Self::Double) => Self::Double,
            (Self::Int64, Self::Int64) => Self::Int64,
            (Self::Bool, Self::Bool) => Self::Bool,
            _ => Self::Text,
        }
    }

    /// Return the column type
    fn sql_type(self) -> &'static str {
        match self {
            Self::Bool | Self::Int64 => "INTEGER",
            Self::Double => "REAL",
            Self::Text => "TEXT",
        }
    }
}

/// Convert the JSON data into supported array of sql data
fn bind_values(schema: &SqliteSchema, row: &Map<String, Value>) -> Vec<SqlValue> {
    schema
        .columns
        .iter()
        .map(|column| match &column.source_name {
            Some(name) => value_for_column(column.kind, row.get(name)),
            None => extra_json(schema, row).map_or(SqlValue::Null, SqlValue::Text),
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
        ColumnKind::Text => value_as_string(json_value).map_or(SqlValue::Null, SqlValue::Text),
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
///
/// Typically will only happen if using `BoaJS` to write custom parsers
/// and the output is **not** consistent
fn extra_json(schema: &SqliteSchema, row: &Map<String, Value>) -> Option<String> {
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

/// Converts a source field name into a unique sqlite column name
fn unique_field_name(source: &str, used: &mut HashSet<String>) -> String {
    let base = sanitize_ident(source);
    let mut candidate = base.clone();
    let mut suffix = 1;

    while used.contains(&candidate) {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }

    used.insert(candidate.clone());
    candidate
}
/// Converts an artifact name into a unique sqlite table name
fn unique_table_name(artifact_name: &str, tables: &HashMap<String, SqliteSchema>) -> String {
    let base = sanitize_ident(artifact_name);
    let mut candidate = base.clone();
    let mut suffix = 1;

    while tables.contains_key(&candidate) {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }

    candidate
}

/// Replaces unsupported sqlite identifier characters with underscores
fn sanitize_ident(source: &str) -> String {
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

/// Quotes a sqlite identifier
fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

/// Convert `rusqlite::Error` to `OutputError`
fn sqlite_error(err: rusqlite::Error) -> OutputError {
    OutputError::Encode(format!("sqlite error: {err}"))
}

/// Convert a path-specific sqlite open error
fn sqlite_path_error(path: impl AsRef<std::path::Path>, err: rusqlite::Error) -> OutputError {
    OutputError::Encode(format!(
        "failed to open sqlite file {}: {err}",
        path.as_ref().display()
    ))
}

#[cfg(test)]
mod tests {
    use super::{SqliteEncoder, sanitize_ident};
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

    #[test]
    fn test_sanitize_ident() {
        assert_eq!(sanitize_ident("eventlogs"), "eventlogs");
        assert_eq!(sanitize_ident("foo/bar"), "foo_bar");
        assert_eq!(sanitize_ident("1start"), "_1start");
        assert_eq!(sanitize_ident(""), "field");
    }
}
