use crate::output2::error::{OutputError, OutputResult};
use serde::Serialize;
use serde_json::{Map, Value};
use std::vec::IntoIter;

/// A JSON backed artifact record represented as a key/value object
#[derive(Debug, PartialEq)]
pub(crate) struct JsonRecord {
    pub(crate) fields: Map<String, Value>,
}

impl JsonRecord {
    /// Create a new JSON backed artifact record from object fields
    pub(crate) fn new(fields: Map<String, Value>) -> Self {
        Self { fields }
    }

    /// Converts the `JsonRecord` back into `serde_json` `Value` object
    pub(crate) fn into_value(self) -> Value {
        Value::Object(self.fields)
    }
}

/// A single artifact entry is represented as a `Record`.
///
/// `Record` describes the internal shape of one artifact entry before it is
/// encoded into an output format.
#[derive(Debug, PartialEq)]
pub(crate) enum Record {
    /// Artifact entry represented as a JSON object.
    Json(JsonRecord),
}

/// Streams artifact records to an output encoder.
pub(crate) trait RecordStream {
    /// Returns the next record, or `None` when the stream is exhausted.
    fn next_record(&mut self) -> OutputResult<Option<Record>>;
}

/// A `RecordStream` backed by a vector of `Record`.
pub(crate) struct VecRecordStream {
    /// Iterator over the remaining records.
    records: IntoIter<Record>,
}

impl VecRecordStream {
    /// Creates a new stream from a vector of records.
    pub(crate) fn new(records: Vec<Record>) -> Self {
        Self {
            records: records.into_iter(),
        }
    }
}

impl RecordStream for VecRecordStream {
    fn next_record(&mut self) -> OutputResult<Option<Record>> {
        Ok(self.records.next())
    }
}

/// Serializes an artifact entry into a JSON backed `Record`.
///
/// The serialized value must be a JSON object so it can be represented as a
/// key/value artifact record.
pub(crate) fn serialize_to_record<T: Serialize>(value: T) -> OutputResult<Record> {
    let value = serde_json::to_value(value)?;
    let fields = match value {
        Value::Object(fields) => fields,
        _ => {
            return Err(OutputError::Record(String::from(
                "serialized artifact record was not a JSON object",
            )));
        }
    };
    Ok(Record::Json(JsonRecord::new(fields)))
}

/// Converts a vector of artifact entries into a vector of `RecordStream`.
///
/// Each entry must serialize to a JSON object so it can be represented as a
/// key/value artifact record.
pub(crate) fn serialize_records_to_stream<T: Serialize>(
    records: Vec<T>,
) -> OutputResult<VecRecordStream> {
    let records = records
        .into_iter()
        .map(serialize_to_record)
        .collect::<OutputResult<Vec<_>>>()?;
    Ok(VecRecordStream::new(records))
}

#[cfg(test)]
mod tests {
    use crate::output2::{
        error::OutputError,
        record::{
            JsonRecord, Record, RecordStream, VecRecordStream, serialize_records_to_stream,
            serialize_to_record,
        },
    };
    use serde_json::{Map, Value, json};

    #[test]
    fn test_json_record() {
        let mut fields = Map::new();
        fields.insert(String::from("path"), "/tmp/test.txt".into());
        fields.insert(String::from("size"), 1234.into());

        let record = JsonRecord::new(fields);
        assert_eq!(record.fields["path"], "/tmp/test.txt");

        let value = record.into_value();
        assert_eq!(value, json!({"path": "/tmp/test.txt", "size": 1234}));
    }

    #[test]
    fn test_record_order() {
        let record_one = JsonRecord::new(Map::from_iter([(
            "path".to_string(),
            Value::String("/tmp/one.txt".to_string()),
        )]));
        let record_two = JsonRecord::new(Map::from_iter([(
            "path".to_string(),
            Value::String("/tmp/two.txt".to_string()),
        )]));
        let mut stream =
            VecRecordStream::new(vec![Record::Json(record_one), Record::Json(record_two)]);
        let first = stream.next_record().unwrap().unwrap();
        let second = stream.next_record().unwrap().unwrap();
        let third = stream.next_record().unwrap();
        match first {
            Record::Json(record) => {
                assert_eq!(record.into_value(), json!({ "path": "/tmp/one.txt" }));
            }
        }
        match second {
            Record::Json(record) => {
                assert_eq!(record.into_value(), json!({ "path": "/tmp/two.txt" }));
            }
        }
        assert!(third.is_none());
    }

    #[test]
    fn test_vec_record_stream_empty() {
        let mut stream = VecRecordStream::new(Vec::new());
        let record = stream.next_record().unwrap();
        assert!(record.is_none());
    }

    #[test]
    fn test_serialize_to_record_bad_value() {
        let test = "test";
        let err = serialize_to_record(Value::String(test.into())).unwrap_err();
        assert!(
            matches!(err, OutputError::Record(value) if value == "serialized artifact record was not a JSON object")
        );
    }

    #[test]
    fn test_serialize_to_record() {
        let test = json!({"test": "value"});
        let result = serialize_to_record(&test).unwrap();
        assert_eq!(
            result,
            Record::Json(JsonRecord {
                fields: test.as_object().unwrap().clone()
            })
        )
    }

    #[test]
    fn test_serialize_records_to_stream() {
        let test = json!([{"test": "value"}, {"test2": "value2"}]);
        let mut result = serialize_records_to_stream(test.as_array().unwrap().to_vec()).unwrap();
        assert_eq!(result.records.len(), 2);
        assert_eq!(
            result.records.next().unwrap(),
            Record::Json(JsonRecord {
                fields: json!({"test": "value"}).as_object().unwrap().clone()
            })
        )
    }

    #[test]
    fn test_serialize_records_to_stream_bad_value() {
        let err = serialize_to_record(Value::Bool(false)).unwrap_err();
        assert!(
            matches!(err, OutputError::Record(value) if value == "serialized artifact record was not a JSON object")
        );
    }
}
