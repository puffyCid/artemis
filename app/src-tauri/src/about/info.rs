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
pub(crate) async fn about_me(path: &str) -> Result<AboutMe, ()> {
    let mut info = AboutMe {
        apollo: env!("CARGO_PKG_VERSION").to_string(),
        tauri: String::from("2.1.0"),
        rust: env!("VERGEN_RUSTC_SEMVER").to_string(),
        build: env!("VERGEN_BUILD_DATE").to_string(),
        resources: Value::Null,
    };

    let result = get_resources().await;
    match result {
        Ok(value) => {
            info.resources = value;
        }
        Err(_) => {}
    }
    Ok(info)
}

/// Get info on the metadata index in opensearch
#[tauri::command]
pub(crate) async fn metadata(path: &str) -> Result<Value, ()> {
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
mod tests {
    use super::{about_me, metadata};
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_about_me() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/timelines/test.db");

        let about = about_me(test_location.to_str().unwrap()).await.unwrap();
        assert!(!about.apollo.is_empty());
    }

    #[tokio::test]
    async fn test_metadata() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/timelines/test22.db");

        let result = metadata(test_location.to_str().unwrap()).await.unwrap();
    }
}
