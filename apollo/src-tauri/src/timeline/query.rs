use log::error;
use serde_json::Value;

use crate::search::query::{timeline, QueryState};

/// Get list timeline entries based on query values
#[tauri::command]
pub(crate) async fn query_timeline(index: &str, state: QueryState) -> Result<Value, ()> {
    let result = match timeline(index, state).await {
        Ok(result) => result,
        Err(err) => {
            error!("[app] could not get timeline entries: {err:?}");
            Value::Null
        }
    };

    Ok(result)
}

#[cfg(test)]
mod tests {
    use crate::{search::query::QueryState, timeline::query::query_timeline};
    use serde_json::json;

    #[tokio::test]
    async fn test_query_timeline() {
        let query = json!({
            "query": {
                "match_all": {}
            }
        });
        let state = QueryState {
            limit: 50,
            offset: 0,
            order_column: String::from("message"),
            order: String::from("asc"),
            query,
        };

        let result = query_timeline("whatever", state).await.unwrap();
        assert!(result.is_object());
    }
}
