use crate::db::query::{timeline, QueryResults, QueryState};
use log::error;

/// Get list timeline entries based on query values
#[tauri::command]
pub(crate) fn query_timeline(path: &str, state: QueryState) -> QueryResults {
    match timeline(path, &state) {
        Ok(result) => result,
        Err(err) => {
            error!("[app] could not get timeline entries: {err:?}");
            QueryResults {
                data: Vec::new(),
                total_rows: 0,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        db::query::{ColumnName, QueryState},
        timeline::query::query_timeline,
    };
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn test_query_timeline() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/timelines/test.db");

        let state = QueryState {
            limit: 50,
            offset: 0,
            filter: json!(""),
            column: ColumnName::Message,
            order: 1,
            order_column: ColumnName::Datetime,
            comparison: 0,
            json_key: String::new(),
        };

        let result = query_timeline(test_location.to_str().unwrap(), state);
        assert_eq!(result.data.len(), 50);
        assert_eq!(result.total_rows, 2208);
    }
}
