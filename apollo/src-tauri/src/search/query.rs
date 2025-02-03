use super::index::setup_client;
use log::error;
use opensearch::http::{response::Response, StatusCode};
use opensearch::nodes::NodesStatsParts;
use opensearch::{indices::IndicesGetParts, Error};
use opensearch::{SearchParts, UpdateParts};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

/// Get a list of all index in `OpenSearch`. Should be one per timeline/sketch
pub(crate) async fn list_indexes() -> Result<Value, Error> {
    let client = setup_client()?;

    let res = client
        .indices()
        .get(IndicesGetParts::Index(&["*"]))
        .send()
        .await?;

    Ok(check_response(res).await)
}

/// Get info on the metadata index
pub(crate) async fn get_metadata() -> Result<Value, Error> {
    let client = setup_client()?;

    let res = client
        .indices()
        .get(IndicesGetParts::Index(&["collection_metadata"]))
        .send()
        .await?;

    Ok(check_response(res).await)
}

/// Get info on `OpenSearch` resources
pub(crate) async fn get_resources() -> Result<Value, Error> {
    let client = setup_client()?;

    let res = client.nodes().stats(NodesStatsParts::None).send().await?;

    Ok(check_response(res).await)
}
#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct QueryState {
    pub(crate) limit: i64,
    pub(crate) offset: i64,
    pub(crate) query: Value,
    pub(crate) order_column: String,
    pub(crate) order: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Query {
    fields: Vec<String>,
    query: String,
}

/// Return entries in our Indexed timeline
pub(crate) async fn timeline(index: &str, state: QueryState) -> Result<Value, Error> {
    let client = setup_client()?;
    let sort = format!("{}:{}", state.order_column, state.order);
    let res = client
        .search(SearchParts::Index(&[index]))
        .from(state.offset)
        .size(state.limit)
        .sort(&[&sort])
        .body(state.query)
        .send()
        .await?;

    Ok(check_response(res).await)
}

/// Tag an entry in `OpenSearch`
pub(crate) async fn tag(index: &str, id: &str, tag: &str) -> Result<Value, Error> {
    let client = setup_client()?;
    let res = client
        .update(UpdateParts::IndexId(index, id))
        .body(json!({"doc":{"tags":tag}}))
        .send()
        .await?;

    Ok(check_response(res).await)
}

/// Get counts of ingested artifacts
pub(crate) async fn artifacts(index: &str) -> Result<Value, Error> {
    let client = setup_client()?;
    let artifacts = json!(
        {
            "aggs": {
                "artifacts": {
                    "terms": {
                        "field": "artifact"
                    }
                }
            }
        }
    );
    let res = client
        .search(SearchParts::Index(&[index]))
        .size(0)
        .body(artifacts)
        .send()
        .await?;

    Ok(check_response(res).await)
}

/// Check to make sure the `OpenSearch` response was 200 Status Code
pub(crate) async fn check_response(res: Response) -> Value {
    let code = res.status_code();

    let body = res
        .text()
        .await
        .unwrap_or(String::from("could not process query response"));

    if code != StatusCode::OK {
        error!("bad opensearch query response: {body}",);
    }

    serde_json::from_str(&body).unwrap_or(Value::Null)
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use crate::search::query::{
        artifacts, get_metadata, get_resources, list_indexes, tag, timeline, QueryState,
    };
    use serde_json::json;

    #[tokio::test]
    async fn test_list_indexes() {
        let test = list_indexes().await.unwrap();
        assert!(test.is_object());
    }

    #[tokio::test]
    async fn test_artifacts() {
        let test = artifacts("test").await.unwrap();
        assert!(test.is_object());
    }

    #[tokio::test]
    async fn test_get_metadata() {
        let test = get_metadata().await.unwrap();
        assert!(test.is_object());
    }

    #[tokio::test]
    async fn test_tag() {
        let test = tag("test", "WurnYpQBg9z4_oJkAw0i", "bad").await.unwrap();
        assert!(test.is_object());
    }

    #[tokio::test]
    async fn test_get_resources() {
        let test = get_resources().await.unwrap();
        assert!(test.is_object());
    }

    #[tokio::test]
    async fn test_timeline() {
        let index = "test";
        let query = json!({
            "query": {
                "match_all": {}
            }
        });
        let state = QueryState {
            limit: 50,
            offset: 0,
            query,
            order_column: String::from("message"),
            order: String::from("asc"),
        };

        let result = timeline(index, state).await.unwrap();
        assert!(result.is_object());
    }
}
