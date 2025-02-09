use super::query::check_response;
use common::system::LoadPerformance;
use home::home_dir;
use opensearch::{
    auth::Credentials,
    cert::CertificateValidation,
    http::{
        transport::{SingleNodeConnectionPool, TransportBuilder},
        Url,
    },
    indices::{IndicesCreateParts, IndicesDeleteParts},
    BulkOperations, BulkParts, Error, OpenSearch,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fs::{read, write};

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

/// Create an Index for timeline data.
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

    set_current_index(name);
    Ok(check_response(res).await)
}

/// Delete the provided index, will delete all data.
pub(crate) async fn delete_index(name: &str) -> Result<Value, Error> {
    let client = setup_client()?;
    let res = client
        .indices()
        .delete(IndicesDeleteParts::Index(&[name]))
        .send()
        .await?;

    Ok(check_response(res).await)
}

/// Upload metadata about collection to metadata index.
pub(crate) async fn upload_metadata(data: &BulkOperations) -> Result<Value, Error> {
    let client = setup_client()?;

    let res = client
        .bulk(BulkParts::Index("collection_metadata"))
        .body(vec![data])
        .send()
        .await?;

    Ok(check_response(res).await)
}

/// Bulk upload data to `OpenSearch`.
pub(crate) async fn upload_data(data: &BulkOperations, name: &str) -> Result<Value, Error> {
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
    let settings = opensearch_settings();

    let builder =
        TransportBuilder::new(SingleNodeConnectionPool::new(Url::parse(&settings.domain)?))
            .auth(Credentials::Basic(settings.user, settings.creds))
            .cert_validation(CertificateValidation::None);

    let transport = builder.build()?;
    Ok(OpenSearch::new(transport))
}

struct OpenSearchCreds {
    user: String,
    creds: String,
    domain: String,
}

/// Read Apollo settings to get `OpenSearch` creds. If none available, assume default creds
fn opensearch_settings() -> OpenSearchCreds {
    let mut info = OpenSearchCreds {
        user: String::from("admin"),
        creds: String::from("Ughsocomplex123567890!"),
        domain: String::from("https://127.0.0.1:9200"),
    };

    let home_path = home_dir();
    if let Some(path) = home_path {
        let settings = settings_path(path.to_str().unwrap_or_default());
        let bytes = match read(&settings) {
            Ok(result) => result,
            Err(_err) => return info,
        };

        let settings_serde: Value = match serde_json::from_slice(&bytes) {
            Ok(result) => result,
            Err(_err) => return info,
        };

        if !settings_serde.is_object() {
            return info;
        }

        info.user = settings_serde["user"]
            .as_str()
            .unwrap_or("admin")
            .to_string();
        info.creds = settings_serde["creds"]
            .as_str()
            .unwrap_or("Ughsocomplex123567890!")
            .to_string();
        info.domain = settings_serde["domain"]
            .as_str()
            .unwrap_or("127.0.0.1")
            .to_string();
    }

    info
}

/// Try to set current index in settings.json file. If the file does not exist thats ok
fn set_current_index(name: &str) {
    let home_path = home_dir();
    if home_path.is_none() {
        return;
    }
    // Unwrap is ok since we check for None above
    let settings = settings_path(home_path.unwrap().to_str().unwrap_or_default());
    let bytes = match read(&settings) {
        Ok(result) => result,
        Err(_err) => return,
    };

    let mut settings_serde: Value = match serde_json::from_slice(&bytes) {
        Ok(result) => result,
        Err(_err) => return,
    };

    if !settings_serde.is_object() {
        return;
    }

    settings_serde["index"] = serde_json::Value::String(name.to_string());
    let _ = write(
        &settings,
        serde_json::to_vec(&settings_serde).unwrap_or_default(),
    );
}

pub(crate) fn get_index() -> String {
    let home_path = home_dir();
    if home_path.is_none() {
        return String::from("test");
    }

    // Unwrap is ok since we check for None above
    let settings = settings_path(home_path.unwrap().to_str().unwrap_or_default());
    let bytes = match read(&settings) {
        Ok(result) => result,
        Err(_err) => return String::from("test"),
    };

    let settings_serde: Value = match serde_json::from_slice(&bytes) {
        Ok(result) => result,
        Err(_err) => return String::from("test"),
    };

    if !settings_serde.is_object() {
        return String::from("test");
    }

    settings_serde["index"]
        .as_str()
        .unwrap_or_default()
        .to_string()
}

/// Get path to settings.json file based on OS
fn settings_path(home: &str) -> String {
    #[cfg(target_os = "linux")]
    let settings = format!("{home}/.local/share/com.puffycid.apollo/settings.json");
    #[cfg(target_os = "windows")]
    let settings = format!("{home}\\AppData\\Local\\com.puffycid.apollo\\settings.json");
    #[cfg(target_os = "macos")]
    let settings = format!("{home}/Library/Application Support/com.puffycid.apollo/settings.json");

    settings
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::opensearch_settings;
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

    #[test]
    fn test_opensearch_settings() {
        let settings = opensearch_settings();
        assert!(!settings.user.is_empty())
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

        let test = upload_data(&ops, "test").await.unwrap();
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
                meta = serde_json::from_value(value.get("collection_metadata").unwrap().clone())
                    .unwrap();
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

        let test = upload_data(&ops, "test").await.unwrap();
        assert!(test.is_object());

        let test_meta = upload_metadata(&ops_meta).await.unwrap();
        assert!(test_meta.is_object());
    }
}
