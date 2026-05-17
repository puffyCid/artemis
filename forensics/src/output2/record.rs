use crate::output2::error::{OutputError, OutputResult};
use serde::Serialize;
use serde_json::{Map, Value};
use std::vec::IntoIter;

#[derive(Debug)]
pub(crate) struct JsonRecord {
    pub(crate) fields: Map<String, Value>,
}

impl JsonRecord {
    pub(crate) fn new(fields: Map<String, Value>) -> Self {
        Self { fields }
    }

    pub(crate) fn into_value(self) -> Value {
        Value::Object(self.fields)
    }
}

#[derive(Debug)]
pub(crate) enum Record {
    Json(JsonRecord),
}

pub(crate) trait RecordStream {
    fn next_record(&mut self) -> OutputResult<Option<Record>>;
}

pub(crate) struct VecRecordStream {
    records: IntoIter<Record>,
}

impl VecRecordStream {
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
    use crate::output2::record::{JsonRecord, Record, RecordStream, VecRecordStream};
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
}
