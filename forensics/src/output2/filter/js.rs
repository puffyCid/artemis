use crate::{
    output2::{
        context::CollectionContext,
        error::{OutputError, OutputResult},
        record::{JsonRecord, Record, RecordStream},
    },
    runtime::run::filter_record,
};
use serde_json::{Value, json};

/// A JavaScript record stream to filter our `Record` values
pub(crate) struct JsFilterRecordStream<'a> {
    /// Stream of artifacts to filter
    ///
    /// Each record is passed to the JavaScript code as a JSON object
    inner: &'a mut dyn RecordStream,
    /// Base64 encoded JavaScript code
    encoded_script: String,
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
    ) -> Self {
        let filter_context = json!({
            "artifact_name": artifact_name,
            "filter_name": filter_name,
            "collection_name": context.collection_name,
            "collection_id": context.collection_id,
            "endpoint_id": context.endpoint_id,
        });
        Self {
            inner,
            encoded_script: encoded_script.to_string(),
            filter_context,
        }
    }
}

impl RecordStream for JsFilterRecordStream<'_> {
    fn next_record(&mut self) -> OutputResult<Option<Record>> {
        loop {
            let Some(record) = self.inner.next_record()? else {
                return Ok(None);
            };

            let Record::Json(record) = record;
            // Excute our JavaScript code using the BoaJS runtime
            let result = filter_record(
                &self.encoded_script,
                record.into_value(),
                self.filter_context.clone(),
            )
            .map_err(|err| OutputError::Record(format!("javascript filter failed: {err:?}")))?;

            // Only JSON objects are supported right now. Any other value is dropped
            if let Value::Object(fields) = result {
                return Ok(Some(Record::Json(JsonRecord::new(fields))));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::output2::{
        config::OutputConfig,
        context::CollectionContext,
        error::OutputError,
        filter::js::JsFilterRecordStream,
        manager::OutputManager,
        record::{JsonRecord, Record, VecRecordStream},
    };
    use serde_json::Map;
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
        let js = JsFilterRecordStream::new(&mut records, "test", "test", "test", &context);
        assert_eq!(js.encoded_script, "test");
        assert_eq!(js.filter_context["collection_name"], "");
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
            .write_artifact("test", String::from("test"), &mut records)
            .unwrap_err();

        assert!(
            matches!(err, OutputError::Record(value) if value == "javascript filter failed: Decode")
        );
    }
}
