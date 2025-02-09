use crate::search::query::{get_metadata, get_resources};
use log::error;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub struct AboutMe {
    apollo: String,
    tauri: String,
    rust: String,
    build: String,
    resources: Value,
}

/// Get basic info about apollo
#[tauri::command]
pub(crate) async fn about_me() -> Result<AboutMe, ()> {
    let mut info = AboutMe {
        apollo: env!("CARGO_PKG_VERSION").to_string(),
        tauri: String::from("2.2.0"),
        rust: env!("VERGEN_RUSTC_SEMVER").to_string(),
        build: env!("VERGEN_BUILD_DATE").to_string(),
        resources: Value::Null,
    };

    if let Ok(value) = get_resources().await {
        info.resources = value;
    }

    Ok(info)
}

/// Get info on the metadata index in opensearch
#[tauri::command]
pub(crate) async fn metadata() -> Result<Value, ()> {
    let meta = match get_metadata().await {
        Ok(result) => result,
        Err(err) => {
            error!("[app] could not get metadata: {err:?}");
            Value::Null
        }
    };

    Ok(meta)
}

#[cfg(test)]
#[cfg(target_os = "linux")]
mod tests {
    use super::{about_me, metadata};

    #[tokio::test]
    async fn test_about_me() {
        let about = about_me().await.unwrap();
        assert!(!about.apollo.is_empty());
    }

    #[tokio::test]
    async fn test_metadata() {
        let result = metadata().await.unwrap();
        assert!(result.is_object());
    }
}
