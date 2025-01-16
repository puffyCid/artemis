use crate::search::query::tag;
use log::error;
use serde_json::Value;

/// Apply a tag to a entry
#[tauri::command]
pub(crate) async fn apply_tag(index: &str, document_id: &str, tag_name: &str) -> Result<Value, ()> {
    let result = match tag(index, document_id, tag_name).await {
        Ok(result) => result,
        Err(err) => {
            error!("[app] could not update tag: {err:?}");
            Value::Null
        }
    };

    Ok(result)
}
