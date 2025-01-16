use crate::search::upload::upload_timeline;
use log::error;
use serde_json::Value;
use tokio::fs::read_dir;

/// Timeline and upload from folder
#[tauri::command]
pub(crate) async fn timeline_and_upload(index: &str, path: &str) -> Result<Value, ()> {
    let mut result = Value::Null;
    if let Ok(mut dir) = read_dir(path).await {
        while let Ok(Some(entry)) = dir.next_entry().await {
            if !entry
                .file_name()
                .to_str()
                .unwrap_or_default()
                .ends_with("jsonl")
            {
                continue;
            }
            let target = entry.path().display().to_string();
            result = match upload_timeline(&target, index).await {
                Ok(result) => result,
                Err(err) => {
                    error!("[apollo] could not timeline and upload from {path}: {err:?}");
                    continue;
                }
            };
        }
    }

    Ok(result)
}
