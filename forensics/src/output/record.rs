use crate::output::error::{OutputError, OutputResult};
use serde::Serialize;
use serde_json::{Map, Number, Value};
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

/// A single `BoaJS` runtime scalar entry
///
/// `ScalarRecord` describes the primitive `BoaJS` entry before it is encoded
/// into an output format
#[derive(Debug, PartialEq)]
pub(crate) enum ScalarRecord {
    /// String value
    Text(String),
    /// Boolean value
    Bool(bool),
    /// Integer value
    Integer(i64),
    /// Unsigned integer value
    UnsignedInteger(u64),
    /// Float value
    Float(f64),
}

impl ScalarRecord {
    /// Builds a `ScalarRecord` from a JSON value
    ///
    /// String becomes `ScalarRecord::Text`, bool becomes `ScalarRecord::Bool`,
    /// Number becomes `ScalarRecord::Integer` or `ScalarRecord::Float` or `ScalarRecord::UnsignedInteger`
    ///
    /// `BigInt` cannot be built from `serde_json::Value` it must be done
    /// at the `BoaJS` value layer
    pub(crate) fn from_value(value: Value) -> OutputResult<Self> {
        match value {
            Value::String(value) => Ok(Self::Text(value)),
            Value::Bool(value) => Ok(Self::Bool(value)),
            Value::Number(value) => Self::from_number(value),
            _ => Err(OutputError::Record(String::from(
                "value was not a scalar record",
            ))),
        }
    }

    /// Convert `ScalarRecord` into JSON value
    ///
    /// Primarily used by JSON/JSONL encoders. Conversion will fail for
    /// records that cannot be represented as valid JSON, such as non-finite float values
    pub(crate) fn into_value(self) -> OutputResult<Value> {
        match self {
            ScalarRecord::Text(value) => Ok(Value::String(value)),
            ScalarRecord::Bool(value) => Ok(Value::Bool(value)),
            ScalarRecord::Integer(value) => Ok(Value::Number(value.into())),
            ScalarRecord::UnsignedInteger(value) => Ok(Value::Number(value.into())),
            ScalarRecord::Float(value) => {
                Number::from_f64(value).map(Value::Number).ok_or_else(|| {
                    OutputError::Record(String::from("float number not a finite JSON number"))
                })
            }
        }
    }

    /// Return short name for the `ScalarRecord` type
    ///
    /// Used for errors/debugging
    pub(crate) fn kind(&self) -> &str {
        match self {
            ScalarRecord::Text(_) => "text",
            ScalarRecord::Bool(_) => "bool",
            ScalarRecord::Integer(_) => "integer",
            ScalarRecord::Float(_) => "float",
            ScalarRecord::UnsignedInteger(_) => "unsignedinteger",
        }
    }

    /// Convert `ScalarRecord` into text value
    pub(crate) fn to_text(&self) -> String {
        match self {
            ScalarRecord::Text(value) => value.clone(),
            ScalarRecord::Bool(value) => value.to_string(),
            ScalarRecord::Integer(value) => value.to_string(),
            ScalarRecord::Float(value) => value.to_string(),
            ScalarRecord::UnsignedInteger(value) => value.to_string(),
        }
    }

    /// Attempt to convert the JavaScript number to proper `ScalarRecord`
    fn from_number(value: Number) -> OutputResult<Self> {
        if let Some(value) = value.as_u64() {
            return Ok(Self::UnsignedInteger(value));
        }

        if let Some(value) = value.as_i64() {
            return Ok(Self::Integer(value));
        }

        if let Some(value) = value.as_f64() {
            return Ok(Self::Float(value));
        }

        Err(OutputError::Record(String::from("unsupported JSON number")))
    }
}

/// A single output entry is represented as a `Record`.
///
/// `Record` describes the internal shape of one output entry before it is
/// encoded into an output format. Rust artifact parsers produce `JsonRecord` values,
/// while the `BoaJS` runtime may produce scalar, array, or null records.
#[derive(Debug, PartialEq)]
pub(crate) enum Record {
    /// Artifact entry represented as a JSON object.
    Json(JsonRecord),
    /// Scalar value produced by the `BoaJS` runtime
    Scalar(ScalarRecord),
    /// Array values produced by the `BoaJS` runtime
    Array(Vec<Record>),
    /// Null value produced by the `BoaJS` runtime
    Null,
}

impl Record {
    /// Builds a `Record` from a JSON value
    ///
    /// JSON objects become `Record::Json`, scalar JSON values become `Record::Scalar`,
    /// arrays become `Record::Array`, and null becomes `Record::Null`
    pub(crate) fn from_value(value: Value) -> OutputResult<Self> {
        match value {
            Value::Null => Ok(Self::Null),
            Value::String(_) | Value::Bool(_) | Value::Number(_) => {
                ScalarRecord::from_value(value).map(Self::Scalar)
            }
            Value::Object(fields) => Ok(Self::Json(JsonRecord::new(fields))),
            Value::Array(values) => values
                .into_iter()
                .map(Record::from_value)
                .collect::<OutputResult<Vec<_>>>()
                .map(Self::Array),
        }
    }

    /// Convert `Record` into JSON value
    ///
    /// Primarily used by JSON/JSONL encoders. Conversion will fail for
    /// records that cannot be represented as valid JSON, such as non-finite float values
    pub(crate) fn into_value(self) -> OutputResult<Value> {
        match self {
            Self::Json(value) => Ok(value.into_value()),
            Self::Scalar(value) => value.into_value(),
            Self::Array(value) => value
                .into_iter()
                .map(Record::into_value)
                .collect::<OutputResult<Vec<_>>>()
                .map(Value::Array),
            Self::Null => Ok(Value::Null),
        }
    }

    /// Return short name for the `Record` type
    ///
    /// Used for errors/debugging
    pub(crate) fn kind(&self) -> &str {
        match self {
            Self::Scalar(value) => value.kind(),
            Self::Json(_) => "json",
            Self::Array(_) => "array",
            Self::Null => "null",
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum RecordStreamKind {
    /// The stream emits a single object. Do not surround with array brackets when encoding to JSON
    Single,
    /// The stream emits an array of objects. Must surround with array brackets when encoding to JSON
    Array,
}

/// Streams artifact records to an output encoder.
pub(crate) trait RecordStream {
    /// Returns the next record, or `None` when the stream is exhausted.
    fn next_record(&mut self) -> OutputResult<Option<Record>>;
    /// Return the stream type we are using
    fn stream_kind(&self) -> RecordStreamKind;
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

    fn stream_kind(&self) -> RecordStreamKind {
        RecordStreamKind::Array
    }
}

/// A `RecordStream` for a single `Record`
pub(crate) struct SingleRecordStream {
    /// A single `Record` to output
    record: Option<Record>,
}

impl SingleRecordStream {
    /// Create a stream for a single `Record`
    pub(crate) fn new(record: Record) -> Self {
        Self {
            record: Some(record),
        }
    }
}

impl RecordStream for SingleRecordStream {
    fn next_record(&mut self) -> OutputResult<Option<Record>> {
        Ok(self.record.take())
    }

    fn stream_kind(&self) -> RecordStreamKind {
        RecordStreamKind::Single
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
    use crate::output::{
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
            Record::Scalar(_) => panic!("not scalar?"),
            Record::Array(_) => panic!("not array"),
            Record::Null => panic!("not null"),
        }
        match second {
            Record::Json(record) => {
                assert_eq!(record.into_value(), json!({ "path": "/tmp/two.txt" }));
            }
            Record::Scalar(_) => panic!("not scalar?"),
            Record::Array(_) => panic!("not array"),
            Record::Null => panic!("not null"),
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
