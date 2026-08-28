use crate::output::encoder::helper::record::{sanitize_name, unique_field_name};
use chrono::DateTime;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

/// Metadata for one column
#[derive(Debug, Clone)]
pub(crate) struct ColumnSpec {
    /// Source JSON field name for the column
    pub(crate) source_name: Option<String>,
    /// The unique column name
    pub(crate) column_name: String,
    /// Column type
    pub(crate) kind: ColumnKind,
}

/// Supported column value types
#[derive(Copy, Clone, Debug, PartialEq)]
pub(crate) enum ColumnKind {
    Bool,
    Int64,
    Double,
    Utf8,
    UnsignedInt64,
    Timestamp,
    Json,
}

/// Create a Schema based on first row of JSON
#[derive(Debug, Clone)]
pub(crate) struct InferredSchema {
    /// Ordered columns
    pub(crate) columns: Vec<ColumnSpec>,
    /// Source field names included in the inferred schema
    pub(crate) known_fields: HashSet<String>,
}

impl InferredSchema {
    /// Infers a schema from the first chunk of artifact rows
    pub(crate) fn new(rows: &[Map<String, Value>]) -> Self {
        let mut order = Vec::new();
        let mut kinds: HashMap<String, ColumnKind> = HashMap::new();

        // Loop through all rows only for the first stream
        // We loop through all of them just incase the first row
        // is missing data. Ex: parent_pid: null
        // Also for BoaJS output. The user controls the entire array
        // so they could have mixed entries
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

            let column_name = unique_field_name(&source_name, &mut used_names);
            let kind = kinds.get(&source_name).copied().unwrap_or(ColumnKind::Utf8);

            columns.push(ColumnSpec {
                source_name: Some(source_name),
                column_name,
                kind,
            });
        }

        columns.push(ColumnSpec {
            source_name: None,
            column_name: unique_field_name("_extra_json", &mut used_names),
            kind: ColumnKind::Utf8,
        });

        Self {
            columns,
            known_fields,
        }
    }
}

impl ColumnKind {
    /// Convert JSON value to `ColumnKind`
    fn from_value(value: &Value) -> Self {
        match value {
            Value::Bool(_) => Self::Bool,
            Value::Number(number) => {
                if number.is_u64() {
                    Self::UnsignedInt64
                } else if number.is_i64() {
                    Self::Int64
                } else if number.is_f64() {
                    Self::Double
                } else {
                    Self::Utf8
                }
            }
            Value::Object(_) => Self::Json,
            Value::String(val) if check_timestamp(val) => Self::Timestamp,
            Value::Null | Value::Array(_) | Value::String(_) => Self::Utf8,
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

fn check_timestamp(value: &str) -> bool {
    // Shortest RFC 3339 datetime is '1970-01-01T00:00:00Z' (20 chars).
    if value.len() < 20 || value.as_bytes().get(10) != Some(&b'T') {
        return false;
    }

    DateTime::parse_from_rfc3339(value).is_ok()
}

/// Converts an artifact name into a unique table name
pub(crate) fn unique_table_name(
    artifact_name: &str,
    tables: &HashMap<String, InferredSchema>,
) -> String {
    let base = sanitize_name(artifact_name);
    let mut candidate = base.clone();
    let mut suffix = 1;

    while tables.contains_key(&candidate) {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }

    candidate
}

/// Try to properly escape quotes
pub(crate) fn quote_identifier(name: &str) -> String {
    format!("\"{}\"", name.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::{ColumnKind, InferredSchema};
    use serde_json::{Map, Value, json};

    fn rows(values: &[Value]) -> Vec<Map<String, Value>> {
        values
            .iter()
            .map(|value| value.as_object().unwrap().clone())
            .collect()
    }

    fn column<'a>(schema: &'a InferredSchema, source: &str) -> &'a super::ColumnSpec {
        schema
            .columns
            .iter()
            .find(|column| column.source_name.as_deref() == Some(source))
            .unwrap_or_else(|| panic!("missing column {source}"))
    }

    #[test]
    fn test_infer_column_kinds() {
        let schema = InferredSchema::new(&rows(&[json!({
            "flag": true,
            "count": 1,
            "ratio": 1.5,
            "path": "/tmp/one",
            "tags": ["a"],
            "meta": {"k": "v"},
            "empty": null,
            "big": u64::MAX
        })]));

        assert_eq!(column(&schema, "flag").kind, ColumnKind::Bool);
        assert_eq!(column(&schema, "count").kind, ColumnKind::Int64);
        assert_eq!(column(&schema, "ratio").kind, ColumnKind::Double);
        assert_eq!(column(&schema, "path").kind, ColumnKind::Utf8);
        assert_eq!(column(&schema, "tags").kind, ColumnKind::Utf8);
        assert_eq!(column(&schema, "meta").kind, ColumnKind::Utf8);
        assert_eq!(column(&schema, "empty").kind, ColumnKind::Utf8);
        assert_eq!(column(&schema, "big").kind, ColumnKind::Utf8);
    }

    #[test]
    fn test_infer_merges_int_and_float_to_double() {
        let schema = InferredSchema::new(&rows(&[json!({"value": 1}), json!({"value": 2.5})]));
        assert_eq!(column(&schema, "value").kind, ColumnKind::Double);
    }

    #[test]
    fn test_infer_merges_mixed_types_to_utf8() {
        let schema = InferredSchema::new(&rows(&[json!({"value": 1}), json!({"value": true})]));
        assert_eq!(column(&schema, "value").kind, ColumnKind::Utf8);
    }

    #[test]
    fn test_infer_first_seen_field_order() {
        let schema = InferredSchema::new(&rows(&[
            json!({"path": "/tmp/one"}),
            json!({"path": "/tmp/two", "size": 2}),
        ]));

        let names: Vec<_> = schema
            .columns
            .iter()
            .map(|column| column.source_name.as_deref())
            .collect();

        assert_eq!(names, vec![Some("path"), Some("size"), None]);
    }

    #[test]
    fn test_infer_sanitizes_and_dedups_names() {
        let schema = InferredSchema::new(&rows(&[
            json!({"foo/bar": 1}),
            json!({"foo_bar": 2, "1start": "x"}),
        ]));

        assert_eq!(column(&schema, "foo/bar").column_name, "foo_bar");
        assert_eq!(column(&schema, "foo_bar").column_name, "foo_bar_1");
        assert_eq!(column(&schema, "1start").column_name, "_1start");
    }

    #[test]
    fn test_infer_appends_extra_json() {
        let schema = InferredSchema::new(&rows(&[json!({"path": "/tmp/one"})]));
        let extra = schema.columns.last().unwrap();

        assert_eq!(extra.source_name, None);
        assert_eq!(extra.column_name, "_extra_json");
        assert_eq!(extra.kind, ColumnKind::Utf8);

        assert!(schema.known_fields.contains("path"));
        assert!(!schema.known_fields.contains("_extra_json"));
    }

    #[test]
    fn test_infer_extra_json_name_collision() {
        let schema = InferredSchema::new(&rows(&[json!({"_extra_json": "value"})]));
        let extra = schema.columns.last().unwrap();

        assert_eq!(column(&schema, "_extra_json").column_name, "_extra_json");
        assert_eq!(extra.source_name, None);
        assert_eq!(extra.column_name, "_extra_json_1");
    }

    #[test]
    fn test_infer_empty_rows() {
        let schema = InferredSchema::new(&[]);

        assert!(schema.known_fields.is_empty());

        assert_eq!(schema.columns.len(), 1);
        assert_eq!(schema.columns[0].source_name, None);
        assert_eq!(schema.columns[0].column_name, "_extra_json");
        assert_eq!(schema.columns[0].kind, ColumnKind::Utf8);
    }
}
