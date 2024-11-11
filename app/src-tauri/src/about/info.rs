use serde::Serialize;

use crate::db::query::about;

#[derive(Serialize)]
pub struct AboutMe {
    artemis: String,
    tauri: String,
    rust: String,
    build: String,
    artifacts: u32,
    files: u32,
    db: u64,
}

/// Get basic info about artemis
#[tauri::command]
pub(crate) fn about_me(path: &str) -> AboutMe {
    let mut info = AboutMe {
        artemis: env!("CARGO_PKG_VERSION").to_string(),
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

#[cfg(test)]
mod tests {
    use super::about_me;
    use std::path::PathBuf;

    #[test]
    fn test_about_me() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/timelines/test.db");

        let about = about_me(test_location.to_str().unwrap());
        assert!(!about.artemis.is_empty());
        assert_eq!(about.db, 2088960);
        assert_eq!(about.artifacts, 1);
    }
}
