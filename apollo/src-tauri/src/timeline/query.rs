use crate::search::query::{artifacts, list_indexes, timeline, QueryState};
use log::error;
use serde_json::Value;

/// Get list timeline entries based on query values
#[tauri::command]
pub(crate) async fn query_timeline(state: QueryState) -> Result<Value, ()> {
    let result = match timeline(state).await {
        Ok(result) => result,
        Err(err) => {
            error!("[app] could not get timeline entries: {err:?}");
            Value::Null
        }
    };

    Ok(result)
}

/// Get count of ingested artifacts
#[tauri::command]
pub(crate) async fn list_artifacts() -> Result<Value, ()> {
    let result = match artifacts().await {
        Ok(result) => result,
        Err(err) => {
            error!("[app] could not get artifacts counts: {err:?}");
            Value::Null
        }
    };

    Ok(result)
}

/// Get list of indexes
#[tauri::command]
pub(crate) async fn indexes() -> Result<Value, ()> {
    let result = match list_indexes().await {
        Ok(result) => result,
        Err(err) => {
            error!("[app] could not get indexes list: {err:?}");
            Value::Null
        }
    };

    Ok(result)
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use crate::{
        search::query::QueryState,
        timeline::query::{list_artifacts, query_timeline},
    };
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

        let result = query_timeline(state).await.unwrap();
        assert!(result.is_object());
    }

    #[tokio::test]
    async fn test_list_artifacts() {
        let result = list_artifacts().await.unwrap();
        assert!(result.is_object());
    }
}
