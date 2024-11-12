use common::system::LoadPerformance;
use rusqlite::{params, Connection, Error, ToSql};
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Setup the default storage tables
pub(crate) fn setup_tables(path: &str) -> Result<(), Error> {
    let tables = [
        "CREATE TABLE IF NOT EXISTS artifacts (row INTEGER PRIMARY KEY, name TEXT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS files (row INTEGER PRIMARY KEY, timestamp TEXT NOT NULL, filename TEXT NOT NULL, path TEXT NOT NULL, size INTEGER NOT NULL, artifact TEXT NOT NULL)",
        "CREATE TABLE IF NOT EXISTS metadata (row INTEGER PRIMARY KEY, endpoint_id TEXT NOT NULL, uuid TEXT NOT NULL, id INTEGER NOT NULL, artifact_name TEXT NOT NULL, complete_time TEXT NOT NULL, start_time TEXT NOT NULL, hostname TEXT NOT NULL, os_version TEXT NOT NULL, kernel_version TEXT NOT NULL, platform TEXT NOT NULL, avg_one_min REAL NOT NULL, avg_five_min REAL NOT NULL, avg_fifteen_min REAL NOT NULL)",
        "CREATE TABLE IF NOT EXISTS timeline (row INTEGER PRIMARY KEY, message TEXT NOT NULL, artifact TEXT NOT NULL, datetime TEXT DEFAULT '1970-01-01T00:00:00' NOT NULL, timestamp_desc TEXT NOT NULL, data_type TEXT NOT NULL, tags TEXT DEFAULT '' NOT NULL, notes TEXT DEFAULT '' NOT NULL, data BLOB NOT NULL)"
    ];

    for table in tables {
        let _size = create_table(table, path)?;
    }

    Ok(())
}

/// Insert a new artifact into the database
pub(crate) fn insert_artifact(name: &str, path: &str) -> Result<(), Error> {
    let param = params![name];
    let query = "INSERT INTO artifacts(name) VALUES (?1)";

    insert_row(query, param, path)?;
    Ok(())
}

#[derive(Serialize)]
pub(crate) struct FileInfo {
    timestamp: String,
    filename: String,
    path: String,
    size: u64,
    artifact: String,
}

/// Insert new file info into the database
pub(crate) fn insert_files(info: &FileInfo, path: &str) -> Result<(), Error> {
    let param = params![
        info.timestamp,
        info.filename,
        info.path,
        info.size,
        info.artifact
    ];

    let query =
        "INSERT INTO files(timestamp, filename, path, size, artifact) VALUES (?1, ?2, ?3, ?4, ?5)";

    insert_row(query, param, path)?;

    Ok(())
}

#[derive(Serialize, Deserialize)]
pub(crate) struct Metadata {
    endpoint_id: String,
    id: u64,
    artifact_name: String,
    complete_time: String,
    start_time: String,
    hostname: String,
    os_version: String,
    platform: String,
    kernel_version: String,
    load_performance: LoadPerformance,
    uuid: String,
}

/// Insert metadata associated with the collection into the databasae
pub(crate) fn insert_metadata(info: &Metadata, path: &str) -> Result<(), Error> {
    let param = params![
        info.endpoint_id,
        info.uuid,
        info.id,
        info.artifact_name,
        info.complete_time,
        info.start_time,
        info.hostname,
        info.os_version,
        info.kernel_version,
        info.platform,
        info.load_performance.avg_one_min,
        info.load_performance.avg_five_min,
        info.load_performance.avg_fifteen_min,
    ];

    let query =
        "INSERT INTO metadata(endpoint_id, uuid, id, artifact_name, complete_time, start_time, hostname, os_version, kernel_version, platform, avg_one_min, avg_five_min, avg_fifteen_min) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)";

    insert_row(query, param, path)?;

    Ok(())
}

/// Insert array of timeline entries into the database
pub(crate) fn insert_timeline(entries: Vec<Value>, path: &str) -> Result<(), Error> {
    let connection = Connection::open(path)?;
    let query = "INSERT INTO timeline(message, datetime, timestamp_desc, artifact, data_type, data) VALUES (?1, ?2, ?3, ?4, ?5, jsonb(?6))";
    let mut cache = connection.prepare_cached(query)?;

    let status = connection.unchecked_transaction()?;
    for entry in entries {
        let params = (
            entry
                .get("message")
                .unwrap_or(&Value::Null)
                .as_str()
                .unwrap_or_default(),
            entry
                .get("datetime")
                .unwrap_or(&Value::Null)
                .as_str()
                .unwrap_or("1970-01-01T00:00:00Z"),
            entry
                .get("timestamp_desc")
                .unwrap_or(&Value::Null)
                .as_str()
                .unwrap_or_default(),
            entry
                .get("artifact")
                .unwrap_or(&Value::Null)
                .as_str()
                .unwrap_or_default(),
            entry
                .get("data_type")
                .unwrap_or(&Value::Null)
                .as_str()
                .unwrap_or_default(),
            entry.to_string(),
        );
        cache.execute(params)?;
    }

    status.commit()?;
    Ok(())
}

/// Create a provided sqlite table at path
fn create_table(table: &str, path: &str) -> Result<usize, Error> {
    let connection = Connection::open(path)?;
    connection.execute(table, ())
}

/// Insert a single row into the database
fn insert_row(query: &str, params: &[&dyn ToSql], path: &str) -> Result<usize, Error> {
    let connection = Connection::open(path)?;
    connection.execute(query, params)
}

#[cfg(test)]
mod tests {
    use super::{
        create_table, insert_artifact, insert_files, insert_metadata, insert_row, insert_timeline,
        setup_tables, FileInfo, Metadata,
    };
    use common::system::LoadPerformance;
    use rusqlite::params;
    use serde_json::Value;
    use std::{
        fs::{create_dir, read_to_string},
        path::PathBuf,
    };

    #[test]
    fn test_setup_tables() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tmp/");
        let _ = create_dir(test_location.to_str().unwrap());

        test_location.push("test.db");

        setup_tables(test_location.to_str().unwrap()).unwrap();
    }

    #[test]
    fn test_create_tables() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tmp/");
        let _ = create_dir(test_location.to_str().unwrap());

        test_location.push("test.db");

        create_table(
            "CREATE TABLE IF NOT EXISTS test (id INTEGER PRIMARY KEY, name TEXT NOT NULL)",
            test_location.to_str().unwrap(),
        )
        .unwrap();
    }

    #[test]
    fn test_insert_artifact() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tmp/");
        let _ = create_dir(test_location.to_str().unwrap());

        test_location.push("test.db");
        setup_tables(test_location.to_str().unwrap()).unwrap();

        insert_artifact("test", test_location.to_str().unwrap()).unwrap()
    }

    #[test]
    fn test_insert_files() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tmp/");
        let _ = create_dir(test_location.to_str().unwrap());

        test_location.push("test.db");
        setup_tables(test_location.to_str().unwrap()).unwrap();

        let test = FileInfo {
            filename: String::from("test.jsonl"),
            path: String::from("./tmp/test.jsonl"),
            timestamp: String::from("2024-11-01T00:00:00Z"),
            size: 1100,
            artifact: String::from("test"),
        };

        insert_files(&test, test_location.to_str().unwrap()).unwrap()
    }

    #[test]
    fn test_insert_metadata() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tmp/");
        let _ = create_dir(test_location.to_str().unwrap());

        test_location.push("test.db");
        setup_tables(test_location.to_str().unwrap()).unwrap();

        let test = Metadata {
            endpoint_id: String::from("1234"),
            uuid: String::from("1-1-1-1-1"),
            id: 1100,
            artifact_name: String::from("test"),
            start_time: String::from("2024-11-11T00:00:00Z"),
            complete_time: String::from("2024-11-12T00:00:00Z"),
            os_version: String::from("Fedora 41"),
            kernel_version: String::from("6.3.1"),
            platform: String::from("linux"),
            hostname: String::from("test"),
            load_performance: LoadPerformance {
                avg_one_min: 0.11,
                avg_five_min: 1.23,
                avg_fifteen_min: 0.34,
            },
        };

        insert_metadata(&test, test_location.to_str().unwrap()).unwrap()
    }

    #[test]
    fn test_insert_timeline_no_metadata() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tmp/");
        let _ = create_dir(test_location.to_str().unwrap());

        test_location.push("test.db");
        let db_path = test_location.clone();
        setup_tables(test_location.to_str().unwrap()).unwrap();

        test_location.pop();
        test_location.pop();
        test_location.push("tests/timelines/no_metadata.jsonl");

        let mut values = Vec::new();

        for line in read_to_string(test_location.to_str().unwrap())
            .unwrap()
            .lines()
        {
            let value: Value = serde_json::from_str(&line).unwrap();
            values.push(value);
        }

        insert_artifact("fsevents", db_path.to_str().unwrap()).unwrap();

        insert_timeline(values, db_path.to_str().unwrap()).unwrap()
    }

    #[test]
    fn test_insert_timeline() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tmp/");
        let _ = create_dir(test_location.to_str().unwrap());

        test_location.push("test.db");
        let binding = test_location.clone();
        let db_path = binding.to_str().unwrap();
        setup_tables(test_location.to_str().unwrap()).unwrap();

        test_location.pop();
        test_location.pop();
        test_location.push("tests/timelines/metadata.jsonl");

        let mut values = Vec::new();
        let mut meta = Metadata {
            platform: String::new(),
            endpoint_id: String::new(),
            uuid: String::new(),
            id: 0,
            artifact_name: String::new(),
            start_time: String::new(),
            complete_time: String::new(),
            os_version: String::new(),
            kernel_version: String::new(),
            hostname: String::new(),
            load_performance: LoadPerformance {
                avg_one_min: 0.0,
                avg_five_min: 0.0,
                avg_fifteen_min: 0.0,
            },
        };

        for line in read_to_string(test_location.to_str().unwrap())
            .unwrap()
            .lines()
        {
            let value: Value = serde_json::from_str(&line).unwrap();
            if meta.uuid.is_empty() {
                meta = serde_json::from_value(value.get("metadata").unwrap().clone()).unwrap();
            }
            values.push(value.get("data").unwrap().clone());
        }

        insert_metadata(&meta, db_path).unwrap();
        let info = FileInfo {
            path: test_location.to_str().unwrap().to_string(),
            filename: String::from("metadata.jsonl"),
            size: 3144436,
            timestamp: String::from("2024-11-10T00:00:00Z"),
            artifact: String::from("fsevents"),
        };
        insert_files(&info, db_path).unwrap();
        insert_artifact("fsevents", db_path).unwrap();

        insert_timeline(values, db_path).unwrap()
    }

    #[test]
    fn test_insert_row() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tmp/");
        let _ = create_dir(test_location.to_str().unwrap());

        test_location.push("test.db");

        setup_tables(test_location.to_str().unwrap()).unwrap();

        insert_row(
            "INSERT INTO artifacts(name) VALUES (?1)",
            params!["anything"],
            test_location.to_str().unwrap(),
        )
        .unwrap();
    }
}
