use crate::{
    output2::{
        context::CollectionContext,
        error::{OutputError, OutputResult},
        record::{JsonRecord, Record, RecordStream, RecordStreamKind},
    },
    runtime::run::{JsFilterRuntime, create_filter_runtime},
    utils::{encoding::base64_decode_standard, strings::extract_utf8_string},
};
use serde_json::{Value, json};

/// A JavaScript record stream to filter our `Record` values
pub(crate) struct JsFilterRecordStream<'a> {
    /// Stream of artifacts to filter
    ///
    /// Each record is passed to the JavaScript code as a JSON object
    inner: &'a mut dyn RecordStream,
    /// A JavaScript runtime we use to filter data
    runtime: JsFilterRuntime,
    /// Some minor metadata to pass to our script
    filter_context: Value,
}

impl<'a> JsFilterRecordStream<'a> {
    /// Creates a new JavaScript stream to filter artifacts
    pub(crate) fn new(
        inner: &'a mut dyn RecordStream,
        encoded_script: &str,
        artifact_name: &str,
        filter_name: &str,
        context: &CollectionContext,
    ) -> OutputResult<Self> {
        let script_bytes = base64_decode_standard(encoded_script).map_err(|err| {
            OutputError::Record(format!("javascript filter decode failed: {err:?}"))
        })?;
        let script = extract_utf8_string(&script_bytes);
        let runtime = create_filter_runtime(&script)
            .map_err(|err| OutputError::Record(format!("javascript filter failed: {err:?}")))?;
        let filter_context = json!({
            "artifact_name": artifact_name,
            "filter_name": filter_name,
            "collection_name": context.collection_name,
            "collection_id": context.collection_id,
            "endpoint_id": context.endpoint_id,
        });
        Ok(Self {
            inner,
            runtime,
            filter_context,
        })
    }
}

impl RecordStream for JsFilterRecordStream<'_> {
    fn next_record(&mut self) -> OutputResult<Option<Record>> {
        loop {
            let Some(record) = self.inner.next_record()? else {
                return Ok(None);
            };

            let Record::Json(record) = record else {
                return Err(OutputError::unsupported_record("filter", record.kind()));
            };
            // Excute our JavaScript code using the BoaJS runtime
            let result = self
                .runtime
                .filter_record(record.into_value(), &self.filter_context)
                .map_err(|err| OutputError::Record(format!("javascript filter failed: {err:?}")))?;

            match result {
                Value::Null => {}
                Value::Object(fields) => {
                    return Ok(Some(Record::Json(JsonRecord::new(fields))));
                }
                other => {
                    return Err(OutputError::Record(format!(
                        "javascript filter must return an object or null; received {}",
                        json_value_kind(&other)
                    )));
                }
            }
        }
    }

    fn stream_kind(&self) -> RecordStreamKind {
        self.inner.stream_kind()
    }
}

/// Return short name for `serde_json::Value`
fn json_value_kind(value: &Value) -> &str {
    match value {
        Value::Null => "null",
        Value::Bool(_) => "bool",
        Value::Number(_) => "number",
        Value::String(_) => "string",
        Value::Array(_) => "array",
        Value::Object(_) => "object",
    }
}

#[cfg(test)]
mod tests {
    use crate::output2::{
        config::OutputConfig,
        context::CollectionContext,
        error::OutputError,
        filter::js::{JsFilterRecordStream, json_value_kind},
        manager::OutputManager,
        record::{JsonRecord, Record, RecordStreamKind, SingleRecordStream, VecRecordStream},
    };
    use serde_json::{Map, Value};
    use std::path::PathBuf;

    #[test]
    fn test_js_stream() {
        let config = OutputConfig::default();
        let context = CollectionContext::new(&config, PathBuf::from("./tmp"));
        let mut first = Map::new();
        first.insert("path".to_string(), "/tmp/one.txt".into());
        first.insert("size".to_string(), 1235.into());
        let mut second = Map::new();
        second.insert("path".to_string(), "/tmp/two.txt".into());
        second.insert("size".to_string(), 5.into());
        let mut records = VecRecordStream::new(vec![
            Record::Json(JsonRecord::new(first)),
            Record::Json(JsonRecord::new(second)),
        ]);
        let js = JsFilterRecordStream::new(&mut records, "YXN5bmMgZnVuY3Rpb24gbWFpbihyZWNvcmQsIGNvbnRleHQpIHsKICBhd2FpdCBQcm9taXNlLnJlc29sdmUoKTsKICBpZihyZWNvcmQucGF0aCAhPT0gIi90bXAvdHdvLnR4dCIpIHsKICAgIHJldHVybiBudWxsOwogIH0KIGNvbnNvbGUubG9nKGBJIGdvdCAke3JlY29yZC5wYXRofWApOwogIGNvbnNvbGUubG9nKGBDb250ZXh0IGlzIGVuZHBvaW50IElEOiAke2NvbnRleHQuZW5kcG9pbnRfaWR9YCk7CiAgcmVjb3JkWyJtZXNzYWdlIl0gPSAiWW91IGdvdCBhc3luYyBmaWx0ZXJlZCEiOwogIHJlY29yZFsiZmlsdGVyZWRfYnkiXSA9IGNvbnRleHQuZmlsdGVyX25hbWU7CiAgcmVjb3JkWyJhc3luY19maWx0ZXIiXSA9IHRydWU7CiAgcmV0dXJuIHJlY29yZDsKfQ==", "test", "test", &context).unwrap();
        assert_eq!(js.filter_context["collection_name"], "");
        assert_eq!(js.inner.stream_kind(), RecordStreamKind::Array);
    }

    #[test]
    fn test_js_stream_object() {
        let config = OutputConfig::default();
        let context = CollectionContext::new(&config, PathBuf::from("./tmp"));
        let mut first = Map::new();
        first.insert("path".to_string(), "/tmp/two.txt".into());
        first.insert("size".to_string(), 1235.into());
        let mut records = SingleRecordStream::new(Record::Json(JsonRecord::new(first)));
        let js = JsFilterRecordStream::new(&mut records, "YXN5bmMgZnVuY3Rpb24gbWFpbihyZWNvcmQsIGNvbnRleHQpIHsKICBhd2FpdCBQcm9taXNlLnJlc29sdmUoKTsKICBpZihyZWNvcmQucGF0aCAhPT0gIi90bXAvdHdvLnR4dCIpIHsKICAgIHJldHVybiBudWxsOwogIH0KIGNvbnNvbGUubG9nKGBJIGdvdCAke3JlY29yZC5wYXRofWApOwogIGNvbnNvbGUubG9nKGBDb250ZXh0IGlzIGVuZHBvaW50IElEOiAke2NvbnRleHQuZW5kcG9pbnRfaWR9YCk7CiAgcmVjb3JkWyJtZXNzYWdlIl0gPSAiWW91IGdvdCBhc3luYyBmaWx0ZXJlZCEiOwogIHJlY29yZFsiZmlsdGVyZWRfYnkiXSA9IGNvbnRleHQuZmlsdGVyX25hbWU7CiAgcmVjb3JkWyJhc3luY19maWx0ZXIiXSA9IHRydWU7CiAgcmV0dXJuIHJlY29yZDsKfQ==", "test", "test", &context).unwrap();
        assert_eq!(js.filter_context["collection_name"], "");
        assert_eq!(js.inner.stream_kind(), RecordStreamKind::Single);
    }

    #[test]
    fn test_js_stream_bad_js() {
        let mut config = OutputConfig::default();
        config.filter_script = Some(String::from("testasdfasdf"));
        config.directory = PathBuf::from("./tmp");
        let mut first = Map::new();
        first.insert("path".to_string(), "/tmp/one.txt".into());
        first.insert("size".to_string(), 1235.into());
        let mut second = Map::new();
        second.insert("path".to_string(), "/tmp/two.txt".into());
        second.insert("size".to_string(), 5.into());
        let mut records = VecRecordStream::new(vec![
            Record::Json(JsonRecord::new(first)),
            Record::Json(JsonRecord::new(second)),
        ]);

        let mut manager = OutputManager::new(config).unwrap();
        let err = manager
            .write_artifact("test", &String::from("test"), &mut records)
            .unwrap_err();

        assert!(
            matches!(err, OutputError::Record(value) if value == "javascript filter failed: ExecuteScript")
        );
    }

    #[test]
    fn test_json_value_kind() {
        let test = vec![
            Value::Null,
            Value::Number(1.into()),
            Value::String("test".into()),
            Value::Object(Map::new()),
            Value::Array(Vec::new()),
            Value::Bool(true),
        ];
        for entry in test {
            assert!(!json_value_kind(&entry).is_empty())
        }
    }
}
