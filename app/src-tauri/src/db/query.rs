use crate::utils::filesystem::size;
use rusqlite::{Connection, Error};

pub(crate) struct AboutQuery {
    pub(crate) artifacts_count: u32,
    pub(crate) files_count: u32,
    pub(crate) db_size: u64,
}

/// Get some basic info about the database
pub(crate) fn about(path: &str) -> Result<AboutQuery, Error> {
    let connection = Connection::open(path)?;
    let artifact_query = "SELECT COUNT(DISTINCT name) as count FROM artifacts";
    let file_query = "SELECT COUNT(filename) as count FROM files";

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

#[cfg(test)]
mod tests {
    use super::about;
    use std::path::PathBuf;

    #[test]
    fn test_insert_metadata() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/timelines/test.db");

        let result = about(test_location.to_str().unwrap()).unwrap();
        assert_eq!(result.artifacts_count, 1);
        assert_eq!(result.files_count, 1);
        assert_eq!(result.db_size, 2088960);
    }
}
