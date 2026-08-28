use crate::output::encoder::helper::record::unique_field_name;
use serde_json::{Map, Value};
use std::collections::{HashMap, HashSet};

/// Metadata for one column
#[derive(Debug)]
pub(crate) struct ColumnSpec {
    /// Source JSON field name for the column
    pub(crate) source_name: Option<String>,
    /// The unique column name
    pub(crate) column_name: String,
    /// Column type
    pub(crate) kind: ColumnKind,
}

/// Supported column value types
#[derive(Copy, Clone, Debug)]
pub(crate) enum ColumnKind {
    Bool,
    Int64,
    Double,
    Utf8,
}

/// Infers a schema from the first chunk of artifact rows
pub(crate) fn infer(rows: &[Map<String, Value>]) -> (Vec<ColumnSpec>, HashSet<String>) {
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

    (columns, known_fields)
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
