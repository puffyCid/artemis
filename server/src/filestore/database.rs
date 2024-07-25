use super::{collections::save_endpoint_collection, error::StoreError};
use common::server::collections::{CollectionInfo, CollectionRequest};
use log::error;
use redb::{Database, Error, ReadableTable, TableDefinition};

/**
 * Save `CollectionInfo` to central `collections.redb` file.
 */
pub(crate) async fn save_collection(
    collection: CollectionRequest,
    db: &Database,
    path: &str,
) -> Result<(), StoreError> {
    let status_result = found_collection(&collection.info.id, db);
    let status = match status_result {
        Ok(result) => result,
        Err(err) => {
            if err.to_string() == "Table 'collections' does not exist" {
                false
            } else {
                return Err(StoreError::DuplicateCollectionId);
            }
        }
    };
    if status {
        return Err(StoreError::DuplicateCollectionId);
    }
    let status = write_db(&collection, db);
    if status.is_err() {
        error!("[server] Could not write collection database");
        return Err(StoreError::WriteFile);
    }

    save_endpoint_collection(&collection, path).await;

    Ok(())
}

/// Get list of all collections from database
pub(crate) async fn get_collections(db: &Database) -> Result<Vec<CollectionRequest>, Error> {
    let read_txn = db.begin_read()?;
    let name: TableDefinition<'_, u64, String> = TableDefinition::new("collections");

    let read_table = read_txn.open_table(name)?;
    let mut collections = Vec::new();
    let mut iter = read_table.iter()?;
    while let Some(Ok((_, entry))) = iter.next() {
        let value_result = serde_json::from_str(&entry.value());
        let value: CollectionRequest = match value_result {
            Ok(result) => result,
            Err(err) => {
                error!("[server] Could not serialize database collection: {err:?}");
                continue;
            }
        };
        collections.push(value);
    }

    Ok(collections)
}

/// Check if collection ID is in REDB database
fn found_collection(id: &u64, db: &Database) -> Result<bool, Error> {
    let read_txn = db.begin_read()?;
    let name: TableDefinition<'_, u64, String> = TableDefinition::new("collections");

    let read_table = read_txn.open_table(name)?;
    let value = read_table.get(id)?;
    if value.is_some() {
        return Ok(true);
    }

    Ok(false)
}

/// Update `CollectionInfo` in database
pub(crate) fn update_info_db(info: &CollectionInfo, database: &Database) -> Result<(), Error> {
    let write_txn = database.begin_write()?;
    {
        let name: TableDefinition<'_, u64, String> = TableDefinition::new("collections");
        let mut table = write_txn.open_table(name)?;
        table.insert(info.id, serde_json::to_string(info).unwrap_or_default())?;
    }

    write_txn.commit()?;
    Ok(())
}

/// Store collection request to central database
fn write_db(collection: &CollectionRequest, database: &Database) -> Result<(), Error> {
    let write_txn = database.begin_write()?;
    {
        let name: TableDefinition<'_, u64, String> = TableDefinition::new("collections");
        let mut table = write_txn.open_table(name)?;
        table.insert(
            collection.info.id,
            serde_json::to_string(collection).unwrap_or_default(),
        )?;
    }

    write_txn.commit()?;
    Ok(())
}

/// Add completed endpoint to collection database
fn add_endpoint_db(
    endpoint_id: &str,
    info: &CollectionInfo,
    database: &Database,
) -> Result<(), Error> {
    let read_txn = database.begin_read()?;
    let name: TableDefinition<'_, u64, String> = TableDefinition::new("collections");

    let read_table = read_txn.open_table(name)?;
    let value = read_table.get(info.id)?;
    if let Some(entry) = value {
        let collect_value = serde_json::from_str(&entry.value());
        let mut serde_data: CollectionRequest = match collect_value {
            Ok(result) => result,
            Err(err) => {
                error!("[server] Could not deserialize collection data: {err:?}");
                return Err(Error::Corrupted(format!(
                    "Could not deserialize collection data: {err:?}"
                )));
            }
        };

        serde_data.targets_completed.insert(endpoint_id.to_string());

        let write_txn = database.begin_write()?;
        {
            let mut table = write_txn.open_table(name)?;
            table.insert(info.id, serde_json::to_string(info).unwrap_or_default())?;
        }

        write_txn.commit()?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{
        filestore::database::{
            add_endpoint_db, get_collections, save_collection, update_info_db, write_db,
        },
        utils::filesystem::create_dirs,
    };
    use common::server::collections::{CollectionInfo, CollectionRequest, Status};
    use redb::Database;
    use std::collections::HashSet;

    #[tokio::test]
    async fn test_save_collection() {
        create_dirs("./tmp/save").await.unwrap();
        let path = "./tmp/save/test.redb";

        let mut targets = HashSet::new();
        targets.insert(String::from("dafasdf"));

        let data = CollectionRequest {
            targets,
            targets_completed: HashSet::new(),
            info: CollectionInfo {
            id: 0,
            name: String::from("test"),
            created: 10,
            status: Status::NotStarted,
            duration: 0,
            start_time: 0,
            collection: String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="), 
        } };

        let db = Database::create(path).unwrap();

        save_collection(data, &db, "./tmp/save").await.unwrap();
    }

    #[tokio::test]
    async fn test_write_db_and_get_list() {
        create_dirs("./tmp/save").await.unwrap();
        let path = "./tmp/save/collections1.redb";

        let mut targets = HashSet::new();
        targets.insert(String::from("dafasdf"));

        let data = CollectionRequest {
            targets,
            targets_completed: HashSet::new(),
            info: CollectionInfo {
            id: 0,
            name: String::from("test"),
            created: 10,
            status: Status::NotStarted,
            duration: 0,
            start_time: 0,
            collection:String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="),
        } };

        let db = Database::create(path).unwrap();

        write_db(&data, &db).unwrap();
        let results = get_collections(&db).await.unwrap();
        assert_eq!(results.len(), 1);
    }

    #[tokio::test]
    async fn test_update_info_db() {
        create_dirs("./tmp").await.unwrap();
        let path = "./tmp/save/test2.redb";

        let mut targets = HashSet::new();
        targets.insert(String::from("dafasdf"));

        let mut data = CollectionRequest {
            targets,
            targets_completed: HashSet::new(),
            info: CollectionInfo {
            id: 0,
            name: String::from("test"),
            created: 10,
            status: Status::NotStarted,
            duration: 0,
            start_time: 0,
            collection: String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="), 
        } };

        let db = Database::create(path).unwrap();

        save_collection(data.clone(), &db, "./tmp/save")
            .await
            .unwrap();
        data.info.status = Status::Finished;
        update_info_db(&data.info, &db).unwrap();
    }

    #[tokio::test]
    async fn test_add_endpoint_db() {
        create_dirs("./tmp/test").await.unwrap();
        let path = "./tmp/test/db.redb";

        let mut targets = HashSet::new();
        targets.insert(String::from("dafasdf"));

        let data = CollectionRequest {
            targets,
            targets_completed: HashSet::new(),
            info: CollectionInfo {
            id: 0,
            name: String::from("test"),
            created: 10,
            status: Status::NotStarted,
            duration: 0,
            start_time: 0,
            collection: String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="), 
        } };

        let db = Database::create(path).unwrap();

        save_collection(data.clone(), &db, "./tmp/test/")
            .await
            .unwrap();
        add_endpoint_db("asdfasdfafsd", &data.info, &db).unwrap();
    }
}
