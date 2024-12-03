use super::query::check_response;
use common::system::LoadPerformance;
use opensearch::{
    http::transport::Transport,
    indices::{IndicesCreateParts, IndicesDeleteParts},
    BulkOperations, BulkParts, Error, OpenSearch,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

#[derive(Serialize, Deserialize)]
pub(crate) struct Metadata {
    endpoint_id: String,
    id: u64,
    artifact_name: String,
    complete_time: String,
    start_time: String,
    hostname: String,
    os_version: String,
    platform: String,
    kernel_version: String,
    load_performance: LoadPerformance,
    uuid: String,
}

/// Create an Index for timeline data. Returns `None` on success
pub(crate) async fn create_index(name: &str) -> Result<Value, Error> {
    let client = setup_client()?;
    let res = client
        .indices()
        .create(IndicesCreateParts::Index(name))
        .body(json!(
            {
                "mappings": {
                    "properties": {
                        "message": {"type": "keyword"},
                        "artifact": {"type": "keyword"},
                        "datetime": {"type": "date"},
                        "timestamp_desc": {"type": "keyword"},
                        "data_type": {"type": "text"},
                        "tags": {"type": "text"},
                        "notes": {"type": "text"},
                    }
                }
            }
        ))
        .send()
        .await?;

    Ok(check_response(res).await)
}

/// Delete the provided index, will delete all data. Returns `None` on success
pub(crate) async fn delete_index(name: &str) -> Result<Value, Error> {
    let client = setup_client()?;
    let res = client
        .indices()
        .delete(IndicesDeleteParts::Index(&[name]))
        .send()
        .await?;

    Ok(check_response(res).await)
}

/// Upload metadata about collection to metadata index. Returns `None` on success
pub(crate) async fn upload_metadata(data: BulkOperations) -> Result<Value, Error> {
    let client = setup_client()?;

    let res = client
        .bulk(BulkParts::Index("metadata"))
        .body(vec![data])
        .send()
        .await?;

    Ok(check_response(res).await)
}

/// Bulk upload data to `OpenSearch`. Returns `None` on success
pub(crate) async fn upload_data(data: BulkOperations, name: &str) -> Result<Value, Error> {
    let client = setup_client()?;
    let res = client
        .bulk(BulkParts::Index(name))
        .body(vec![data])
        .send()
        .await?;

    Ok(check_response(res).await)
}

/// Setup the `OpenSearch` client to make requests
pub(crate) fn setup_client() -> Result<OpenSearch, Error> {
    let transport = Transport::single_node("http://192.168.1.193:9200")?;
    Ok(OpenSearch::new(transport))
}

#[cfg(test)]
mod tests {
    use crate::search::index::{create_index, delete_index, upload_data, upload_metadata};
    use opensearch::{BulkOperation, BulkOperations};
    use serde_json::Value;
    use std::fs::read_to_string;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_create_index() {
        let test = create_index("test").await.unwrap();
        assert!(test.is_object());
    }

    #[tokio::test]
    async fn test_delete_index() {
        let test = delete_index("test").await.unwrap();
        assert!(test.is_object());
    }

    #[tokio::test]
    async fn test_upload_data_no_metadata() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/timelines/no_metadata.jsonl");

        let mut ops = BulkOperations::new();

        for line in read_to_string(test_location.to_str().unwrap())
            .unwrap()
            .lines()
        {
            let mut value: Value = serde_json::from_str(&line).unwrap();
            value["timeline_source"] = Value::String(test_location.to_str().unwrap().to_string());
            ops.push(BulkOperation::index(value)).unwrap();
        }

        let test = upload_data(ops, "test").await.unwrap();
        assert!(test.is_object());
    }

    #[tokio::test]
    async fn test_upload_data_metadata() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/timelines/metadata.jsonl");

        let mut meta = Value::Null;
        let mut ops_meta = BulkOperations::new();
        let mut ops = BulkOperations::new();

        for line in read_to_string(test_location.to_str().unwrap())
            .unwrap()
            .lines()
        {
            let mut value: Value = serde_json::from_str(&line).unwrap();

            if meta.is_null() {
                meta = serde_json::from_value(value.get("metadata").unwrap().clone()).unwrap();
                meta["timeline_source"] =
                    Value::String(test_location.to_str().unwrap().to_string());
                ops_meta.push(BulkOperation::index(&meta)).unwrap();
            }
            let data = value["data"].as_object_mut().unwrap();
            data.insert(
                String::from("timeline_source"),
                Value::String(test_location.to_str().unwrap().to_string()),
            );
            ops.push(BulkOperation::index(data)).unwrap();
        }

        let test = upload_data(ops, "test").await.unwrap();
        assert!(test.is_object());

        let test_meta = upload_metadata(ops_meta).await.unwrap();
        assert!(test_meta.is_object());
    }
}
