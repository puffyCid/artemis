use crate::utils::filesystem::size;
use rusqlite::{Connection, Error, OpenFlags};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};

pub(crate) struct AboutQuery {
    pub(crate) artifacts_count: u32,
    pub(crate) files_count: u32,
    pub(crate) db_size: u64,
}

/// Get some basic info about the database
pub(crate) fn about(path: &str) -> Result<AboutQuery, Error> {
    let connection = query_connection(path)?;
    let artifact_query = "SELECT COUNT(DISTINCT name) AS count FROM artifacts";
    let file_query = "SELECT COUNT(filename) AS count FROM files";

    let mut statement = connection.prepare(artifact_query)?;
    let mut rows = statement.query(())?;

    let mut about = AboutQuery {
        artifacts_count: 0,
        files_count: 0,
        db_size: size(path),
    };
    while let Some(row) = rows.next()? {
        let value = row.get_ref("count")?;
        about.artifacts_count = value.as_i64()? as u32;
        break;
    }

    let mut statement = connection.prepare(file_query)?;
    let mut rows = statement.query(())?;
    while let Some(row) = rows.next()? {
        let value = row.get_ref("count")?;
        about.files_count = value.as_i64()? as u32;
        break;
    }

    Ok(about)
}

/// Get list of artifacts in the database
pub(crate) fn artifact_list(path: &str) -> Result<Vec<String>, Error> {
    let connection = query_connection(path)?;
    let query = "SELECT DISTINCT name FROM artifacts";

    let mut statement = connection.prepare(query)?;
    let mut rows = statement.query(())?;

    let mut artifacts = Vec::new();
    while let Some(row) = rows.next()? {
        let value = row.get_ref("name")?;
        artifacts.push(value.as_str()?.to_string())
    }

    Ok(artifacts)
}

#[derive(Serialize, Deserialize)]
/// Kind of inspired by `https://vincjo.fr/datatables/docs/server/getting-started/overview`
pub(crate) struct QueryState {
    pub(crate) limit: u16,
    pub(crate) offset: u64,
    pub(crate) filter: Value,
    pub(crate) column: ColumnName,
    pub(crate) order: u8,
    pub(crate) order_column: ColumnName,
    pub(crate) comparison: u8,
    pub(crate) json_key: String,
}

#[derive(Serialize, Deserialize)]
pub(crate) enum ColumnName {
    Message,
    Artifact,
    Datetime,
    TimestampDesc,
    DataType,
    Tags,
    Notes,
    Data,
}

/// Get array of timeline entries from database
pub(crate) fn timeline(path: &str, state: &QueryState) -> Result<Vec<Map<String, Value>>, Error> {
    let connection = query_connection(path)?;
    let ordering = if state.order == 1 { "ASC" } else { "DSC" };
    let compare = if state.comparison == 1 {
        "= (?1)"
    } else {
        "LIKE '%' || (?1) || '%'"
    };
    let col = get_column_name(&state.column, &state.json_key);
    let order_col = get_column_name(&state.order_column, &state.json_key);
    // This should be ok querying data. All values we are inserting via format we control
    let query = format!("SELECT message, artifact, datetime, timestamp_desc, data_type, tags, notes, JSON(data) AS data FROM timeline WHERE {col} {compare} ORDER BY {order_col} {ordering} LIMIT (?2) OFFSET (?3)");
    let mut statement = connection.prepare(&query)?;
    let column_count = statement.column_count();

    let mut rows = if state.filter.is_string() {
        statement.query((
            &state.filter.as_str().unwrap_or_default(),
            state.limit,
            state.offset,
        ))?
    } else {
        statement.query((&state.filter, state.limit, state.offset))?
    };
    let mut data = Vec::new();

    // Loop through all rows based on provided query
    while let Some(row) = rows.next()? {
        let mut json_data = Map::new();
        // Loop through each column and grab the data
        for column in 0..column_count {
            let value = row.get_ref(column)?;
            let name = row.as_ref().column_name(column)?;
            json_data.insert(name.to_string(), json!(value.as_str()?));
        }
        data.push(json_data);
    }

    Ok(data)
}

/// Determine the column name provided in the queyr
fn get_column_name(name: &ColumnName, key: &str) -> String {
    match name {
        ColumnName::Message => String::from("message"),
        ColumnName::Artifact => String::from("artifact"),
        ColumnName::Datetime => String::from("datetime"),
        ColumnName::TimestampDesc => String::from("timestamp_desc"),
        ColumnName::DataType => String::from("data_type"),
        ColumnName::Tags => String::from("tags"),
        ColumnName::Notes => String::from("notes"),
        ColumnName::Data => format!("JSON_EXTRACT(data, '{}')", key),
    }
}

/// Open the database. All queries are read only
fn query_connection(path: &str) -> Result<Connection, Error> {
    Connection::open_with_flags(
        &format!("file:{path}?immutable=1"),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_URI,
    )
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use crate::db::query::{
        about, artifact_list, get_column_name, timeline, ColumnName, QueryState,
    };
    use std::path::PathBuf;

    #[test]
    fn test_about() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/timelines/test.db");

        let result = about(test_location.to_str().unwrap()).unwrap();
        assert_eq!(result.artifacts_count, 1);
        assert_eq!(result.files_count, 1);
        assert_eq!(result.db_size, 2088960);
    }

    #[test]
    fn test_artifact_list() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/timelines/test.db");

        let result = artifact_list(test_location.to_str().unwrap()).unwrap();
        assert_eq!(result.len(), 1);
        assert_eq!(result[0], "fsevents");
    }

    #[test]
    fn test_get_column_name() {
        let test = [
            ColumnName::Artifact,
            ColumnName::Message,
            ColumnName::Data,
            ColumnName::DataType,
            ColumnName::Datetime,
            ColumnName::Tags,
            ColumnName::Notes,
            ColumnName::TimestampDesc,
        ];

        for entry in test {
            let result = get_column_name(&entry, "");
            assert!(!result.is_empty())
        }
    }

    #[test]
    fn test_timeline() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/timelines/test.db");

        let state = QueryState {
            limit: 5,
            offset: 0,
            filter: json!(""),
            column: ColumnName::Message,
            order: 1,
            order_column: ColumnName::Datetime,
            comparison: 0,
            json_key: String::new(),
        };

        let result = timeline(test_location.to_str().unwrap(), &state).unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(
            result[0].get("message").unwrap().as_str().unwrap(),
            "/Volumes/Preboot"
        );
        assert_eq!(
            result[1].get("artifact").unwrap().as_str().unwrap(),
            "FsEvents"
        );
        assert_eq!(
            result[2].get("datetime").unwrap().as_str().unwrap(),
            "2024-07-25T23:48:13.000Z"
        );
        assert_eq!(
            result[3].get("data_type").unwrap().as_str().unwrap(),
            "macos:fsevents:entry"
        );
        assert_eq!(result[4].get("data").unwrap().to_string().len(), 869);
    }

    #[test]
    fn test_timeline_json_query() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/timelines/test.db");

        let state = QueryState {
            limit: 5,
            offset: 0,
            filter: json!(163140),
            column: ColumnName::Data,
            order: 1,
            order_column: ColumnName::Datetime,
            comparison: 1,
            json_key: String::from("$.event_id"),
        };

        let result = timeline(test_location.to_str().unwrap(), &state).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(
            result[0].get("message").unwrap().as_str().unwrap(),
            "/Volumes/Preboot"
        );
        assert_eq!(
            result[1].get("timestamp_desc").unwrap().as_str().unwrap(),
            "Source Changed"
        );
        assert_eq!(
            result[2].get("datetime").unwrap().as_str().unwrap(),
            "2024-11-10T04:39:20.000Z"
        );
    }

    #[test]
    fn test_timeline_json_array_query() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/timelines/test.db");

        let state = QueryState {
            limit: 5,
            offset: 0,
            filter: json!(["Created", "Renamed", "Modified", "IsFile"]),
            column: ColumnName::Data,
            order: 1,
            order_column: ColumnName::Datetime,
            comparison: 1,
            json_key: String::from("$.flags"),
        };

        let result = timeline(test_location.to_str().unwrap(), &state).unwrap();
        assert_eq!(result.len(), 5);
        assert_eq!(
            result[0].get("message").unwrap().as_str().unwrap(),
            "/Volumes/Preboot/0A81F3B1-51D9-3335-B3E3-169C3640360D/System/Library/Caches/com.apple.corestorage/.dat.nosync007f.9a5dgx"
        );
        assert_eq!(
            result[1].get("timestamp_desc").unwrap().as_str().unwrap(),
            "Source Created Source Modified"
        );
        assert_eq!(
            result[2].get("datetime").unwrap().as_str().unwrap(),
            "2024-07-25T23:48:13.000Z"
        );
    }
}
