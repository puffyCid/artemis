use crate::output::{
    context::ArtifactContext,
    encoder::metadata::append_metadata,
    error::{OutputError, OutputResult},
    record::{Record, RecordStream},
};
use serde_json::{Map, Value};
use std::collections::HashSet;

/// Converts JSON record values into rows.
///
/// `RecordStream` must be an array of JSON.
pub(crate) fn read_json_rows(
    records: &mut dyn RecordStream,
    context: &ArtifactContext,
    format: &str,
) -> OutputResult<Vec<Map<String, Value>>> {
    let mut rows = Vec::new();

    while let Some(record) = records.next_record()? {
        let Record::Json(record) = record else {
            return Err(OutputError::UnsupportedRecord {
                format: format.to_string(),
                record_type: record.kind().to_string(),
            });
        };

        let mut value = record.into_value();
        append_metadata(&mut value, context);
        let Value::Object(fields) = value else {
            return Err(OutputError::Encode(format!(
                "{format} records must be JSON objects",
            )));
        };

        rows.push(fields);
    }

    Ok(rows)
}

/// Replaces unsupported characters with underscores
pub(crate) fn sanitize_name(source: &str) -> String {
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

/// Attempt to convert JSON value to string
pub(crate) fn value_as_string(value: &Value) -> Option<String> {
    match value {
        Value::Null => None,
        Value::String(val) => Some(val.clone()),
        Value::Bool(val) => Some(val.to_string()),
        Value::Number(val) => Some(val.to_string()),
        Value::Array(val) => serde_json::to_string(val).ok(),
        Value::Object(val) => serde_json::to_string(val).ok(),
    }
}

/// Attempt to convert JSON value to integer
pub(crate) fn value_as_i64(value: &Value) -> Option<i64> {
    let number = value.as_number()?;
    if let Some(val) = number.as_i64() {
        return Some(val);
    }

    let value = number.as_u64()?;
    i64::try_from(value).ok()
}

#[cfg(feature = "duck")]
/// Attempt to convert JSON value to unsigned integer
pub(crate) fn value_as_u64(value: &Value) -> Option<u64> {
    let number = value.as_number()?;
    number.as_u64()
}

/// Converts a source field name into a unique column name
pub(crate) fn unique_field_name(source: &str, used: &mut HashSet<String>) -> String {
    let base = sanitize_name(source);
    let mut candidate = base.clone();
    let mut suffix = 1;

    while used.contains(&candidate) {
        candidate = format!("{base}_{suffix}");
        suffix += 1;
    }

    used.insert(candidate.clone());
    candidate
}

/// Serializes fields not present in the inferred schema into the `_extra_json` column
///
/// Typically will only happen if using `BoaJS` to write custom parsers
/// and the output is **not** consistent
pub(crate) fn extra_json(schema: &HashSet<String>, row: &Map<String, Value>) -> Option<String> {
    let mut extra = Map::new();
    for (key, value) in row {
        if !schema.contains(key) {
            extra.insert(key.clone(), value.clone());
        }
    }

    if extra.is_empty() {
        return None;
    }

    serde_json::to_string(&Value::Object(extra)).ok()
}

#[cfg(test)]
mod tests {
    use crate::{
        output::{
            context::CollectionContext,
            encoder::helper::record::{
                extra_json, read_json_rows, sanitize_name, unique_field_name, value_as_string,
            },
            error::OutputError,
            record::{JsonRecord, Record, ScalarRecord, VecRecordStream},
        },
        structs::toml::OutputConfig,
    };
    use serde_json::{Map, Value, json};
    use std::{collections::HashSet, path::PathBuf};

    fn json_record(value: Value) -> Record {
        Record::Json(JsonRecord::new(value.as_object().unwrap().clone()))
    }

    #[test]
    fn test_read_json_rows() {
        let mut first = VecRecordStream::new(vec![json_record(json!({
            "path": "/tmp/one",
            "size": 1
        }))]);

        let output = OutputConfig::default();
        let context = CollectionContext::new(&output, PathBuf::from("./tmp/sqlite_test.log"))
            .artifact("test", &None, &None);

        let values = read_json_rows(&mut first, &context, "test").unwrap();
        assert_eq!(values.len(), 1);
    }

    #[test]
    fn test_read_json_bad_rows() {
        let mut value = VecRecordStream::new(vec![Record::Scalar(ScalarRecord::Integer(5))]);

        let output = OutputConfig::default();
        let context = CollectionContext::new(&output, PathBuf::from("./tmp/sqlite_test.log"))
            .artifact("test", &None, &None);

        let err = read_json_rows(&mut value, &context, "test").unwrap_err();
        assert!(
            matches!(err, OutputError::UnsupportedRecord { format, record_type } if format == "test" && record_type == "integer")
        );
    }

    #[test]
    fn test_sanitize_name() {
        assert_eq!(sanitize_name("!abc"), "_abc");
        assert_eq!(sanitize_name(""), "field");
        assert_eq!(sanitize_name("abc_"), "abc_");
        assert_eq!(sanitize_name("_"), "_");
        assert_eq!(sanitize_name("1"), "_1");
    }

    #[test]
    fn test_value_as_string() {
        assert_eq!(value_as_string(&Value::Null), None);

        let test = [
            Value::String("test".into()),
            Value::Number(1.into()),
            Value::Bool(true),
            Value::Array(Vec::new()),
            Value::Object(Map::new()),
        ];

        for entry in test {
            assert_ne!(value_as_string(&entry), None);
        }
    }

    #[test]
    fn test_unique_field_name() {
        let name = "test";
        let mut used = HashSet::new();
        used.insert(String::from("test"));

        assert_eq!(unique_field_name(name, &mut used), "test_1")
    }

    #[test]
    fn test_extra_json() {
        let test = HashSet::new();
        let value = json!({
            "path": "/tmp/one",
            "size": 1
        });

        let result = extra_json(&test, value.as_object().unwrap()).unwrap();
        assert_eq!(result, "{\"path\":\"/tmp/one\",\"size\":1}");
    }
}
