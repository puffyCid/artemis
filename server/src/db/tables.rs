use super::error::DbError;
use crate::utils::filesystem::is_file;
use log::{error, warn};
use redb::{Database, DatabaseError, ReadableTable, TableDefinition, TableError, WriteTransaction};
use std::{sync::Arc, thread::sleep, time::Duration};

/**
 * Before writing to database we may want to first get existing data before overwriting. This function locks the database.
 * If you **don't** care about existing data use `add_table_data`
 */
pub(crate) fn check_write<'a>(
    db: &'a Database,
    id: &str,
    table_name: &str,
) -> Result<(WriteTransaction<'a>, Vec<u8>), DbError> {
    let begin_result = db.begin_write();
    let write_start = match begin_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to start {table_name} DB write: {err:?}");
            return Err(DbError::BeginWrite);
        }
    };

    let data = lookup_table_data(table_name, id, db)?;
    Ok((write_start, data))
}

/// Add data to a provide table and path
pub(crate) fn add_table_data(
    db: &Database,
    id: &str,
    data: &[u8],
    table_name: &str,
) -> Result<(), DbError> {
    let begin_result = db.begin_write();
    let write_start = match begin_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to start {table_name} DB write: {err:?}");
            return Err(DbError::BeginWrite);
        }
    };

    write_table_data(write_start, id, data, table_name)
}

/// Write data to a provided table name
pub(crate) fn write_table_data(
    write_start: WriteTransaction<'_>,
    id: &str,
    data: &[u8],
    table_name: &str,
) -> Result<(), DbError> {
    let table: TableDefinition<'_, &str, &[u8]> = TableDefinition::new(table_name);

    // Open the table for writing
    {
        let table_result = write_start.open_table(table);
        let mut table_write = match table_result {
            Ok(result) => result,
            Err(err) => {
                error!("[server] Failed to open {table_name} DB table for writing: {err:?}");
                return Err(DbError::OpenTable);
            }
        };

        let result = table_write.insert(id, data);
        match result {
            Ok(_) => {}
            Err(err) => {
                error!("[server] Failed to insert data into {table_name} DB table: {err:?}");
                return Err(DbError::Insert);
            }
        }
    }

    let commit_result = write_start.commit();
    if commit_result.is_err() {
        error!(
            "[server] Failed to commit data into {table_name} DB table: {:?}",
            commit_result.unwrap_err()
        );
        return Err(DbError::Commit);
    }

    Ok(())
}

/// Get table data at path based on ID. Empty data means no value was found
pub(crate) fn lookup_table_data(
    table_name: &str,
    id: &str,
    db: &Database,
) -> Result<Vec<u8>, DbError> {
    let table: TableDefinition<'_, &str, &[u8]> = TableDefinition::new(table_name);

    let begin_result = db.begin_read();
    let read_start = match begin_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to start {table_name} DB read: {err:?}");
            return Err(DbError::BeginRead);
        }
    };

    let table_result = read_start.open_table(table);
    let table_read = match table_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to open {table_name} DB table for reading: {err:?}");
            if let TableError::TableDoesNotExist(_) = err {
                // If table does not exist yet thats ok just return empty data
                return Ok(Vec::new());
            }
            return Err(DbError::OpenTable);
        }
    };

    let read_result = table_read.get(id);
    let data_value = match read_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to get {table_name} DB data: {err:?}");
            return Err(DbError::Get);
        }
    };

    if let Some(value) = data_value {
        let db_data = value.value();
        return Ok(db_data.to_vec());
    }

    Ok(Vec::new())
}

/// Open a database file at provided path
pub(crate) fn setup_db(path: &str) -> Result<Arc<Database>, DbError> {
    let db_result = if !is_file(path) {
        Database::create(path)
    } else {
        Database::open(path)
    };

    let db = match db_result {
        Ok(result) => result,
        Err(err) => {
            // Open errors should only occur during tests. When the server is running the Database is opened and should be shared via axum::State
            if let DatabaseError::DatabaseAlreadyOpen = err {
                let sleep_time = 2;
                warn!("[server] {path} already opened. Sleeping {sleep_time} millisecond(s)");
                sleep(Duration::from_millis(sleep_time));
                return setup_db(path);
            } else {
                println!("[server] Failed to open {path} DB: {err:?}");
                return Err(DbError::Open);
            }
        }
    };

    Ok(Arc::new(db))
}

#[cfg(test)]
mod tests {
    use super::{add_table_data, check_write, lookup_table_data, setup_db, write_table_data};
    use crate::artifacts::enrollment::EndpointDb;
    use std::path::PathBuf;

    #[test]
    fn test_add_table_data() {
        let path = "./tmp/endpointsadd.redb";

        let db = setup_db(path).unwrap();
        let id = "arandomkey";

        add_table_data(&db, id, &[1, 2, 3, 4], "endpoints").unwrap();
    }

    #[test]
    fn test_check_write() {
        let path = "./tmp/jobscheck.redb";

        let db = setup_db(path).unwrap();
        let id = "arandomkey";

        check_write(&db, id, "jobs").unwrap();
    }

    #[test]
    fn test_write_table_data() {
        let path = "./tmp/jobswrite.redb";

        let db = setup_db(path).unwrap();
        let id = "arandomkey";

        let (write_start, _) = check_write(&db, id, "jobs").unwrap();

        write_table_data(write_start, id, &[1, 2, 3, 4, 5], "jobs").unwrap();
    }

    #[test]
    fn test_lookup_table_data() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/endpoints.redb");
        let path = test_location.display().to_string();

        let id = "3482136c-3176-4272-9bd7-b79f025307d6";
        let db = setup_db(&path).unwrap();

        let result = lookup_table_data("endpoints", id, &db).unwrap();
        let endpoint_serde: EndpointDb = serde_json::from_slice(&result).unwrap();

        assert_eq!(endpoint_serde.hostname, "aStudio.lan");
        assert_eq!(endpoint_serde.platform, "Darwin");
        assert_eq!(endpoint_serde.checkin, 1693968058);
    }

    #[test]
    fn test_setup_db() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/endpoints.redb");
        let path = test_location.display().to_string();

        let _ = setup_db(&path).unwrap();
    }
}
