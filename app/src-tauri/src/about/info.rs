use crate::db::query::{about, artifact_list};
use log::error;
use serde::Serialize;

#[derive(Serialize)]
pub struct AboutMe {
    apollo: String,
    tauri: String,
    rust: String,
    build: String,
    artifacts: u32,
    files: u32,
    db: u64,
}

/// Get basic info about apollo
#[tauri::command]
pub(crate) fn about_me(path: &str) -> AboutMe {
    let mut info = AboutMe {
        apollo: env!("CARGO_PKG_VERSION").to_string(),
        tauri: String::from("2.1.0"),
        rust: env!("VERGEN_RUSTC_SEMVER").to_string(),
        build: env!("VERGEN_BUILD_DATE").to_string(),
        artifacts: 0,
        files: 0,
        db: 0,
    };

    let result = about(path);
    match result {
        Ok(value) => {
            info.db = value.db_size;
            info.artifacts = value.artifacts_count;
            info.files = value.files_count;
        }
        Err(_) => {}
    }
    info
}

/// Get list of artifacts ingested into the database
#[tauri::command]
pub(crate) fn artifacts(path: &str) -> Vec<String> {
    match artifact_list(path) {
        Ok(result) => result,
        Err(err) => {
            error!("[app] could not get artifact list: {err:?}");
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{about_me, artifacts};
    use std::path::PathBuf;

    #[test]
    fn test_about_me() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/timelines/test.db");

        let about = about_me(test_location.to_str().unwrap());
        assert!(!about.apollo.is_empty());
        assert_eq!(about.db, 2088960);
        assert_eq!(about.artifacts, 1);
    }

    #[test]
    fn test_artifacts() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/timelines/test22.db");

        let result = artifacts(test_location.to_str().unwrap());
        assert!(result.is_empty());
    }
}
