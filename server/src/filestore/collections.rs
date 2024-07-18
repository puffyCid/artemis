use super::error::StoreError;
use crate::utils::filesystem::{append_file, is_file, read_lines};
use common::server::collections::{CollectionInfo, CollectionRequest, Status};
use log::error;
use redb::{Database, Error, TableDefinition};

/**
 * Save `CollectionInfo` to central `collections.redb` file.
 */
pub(crate) async fn save_collection(
    collection: CollectionRequest,
    db: &Database,
) -> Result<(), StoreError> {
    let status = write_db(&collection, db);
    if status.is_err() {
        error!("[server] Could not write collection database");
        return Err(StoreError::WriteFile);
    }

    Ok(())
}

/// Save collection info associated with endpoint
pub(crate) async fn save_endpoint_collection(
    collection: &CollectionInfo,
    path: &str,
) -> Result<(), StoreError> {
    let collect_file = format!("{path}/collections.jsonl");

    let serde_result = serde_json::to_string(&collection);
    let value = match serde_result {
        Ok(result) => result,
        Err(err) => {
            error!("[server] Failed to serialize endpoint collection data: {err:?}");
            return Err(StoreError::Serialize);
        }
    };

    let limit = 1024 * 1024 * 1024 * 5;

    let status = append_file(&value, &collect_file, &limit).await;
    if status.is_err() {
        error!("[server] Could not write endpoint collection file");
        return Err(StoreError::WriteFile);
    }

    Ok(())
}

/// Return all Collections for endpoint. Path is full path to endpoint **including** the endpoint ID
pub(crate) async fn get_endpoint_collections(
    path: &str,
) -> Result<Vec<CollectionInfo>, StoreError> {
    let collect_file = format!("{path}/collections.jsonl");

    if !is_file(&collect_file) {
        return Ok(Vec::new());
    }

    let mut collects = Vec::new();

    let value_result = read_lines(&collect_file).await;
    if let Ok(mut value) = value_result {
        while let Ok(line) = value.next_line().await {
            if let Some(collect) = line {
                let serde_value = serde_json::from_str(&collect);
                let info: CollectionInfo = match serde_value {
                    Ok(result) => result,
                    Err(err) => {
                        error!("[server] Failed to deserialize collection file data: {err:?}");
                        return Err(StoreError::Deserialize);
                    }
                };

                collects.push(info);
                continue;
            }
            break;
        }
    }

    Ok(collects)
}

/// Return not-started Collections for endpoint. Path is full path to endpoint **including** the endpoint ID
pub(crate) async fn get_endpoint_collections_notstarted(
    path: &str,
) -> Result<Vec<CollectionInfo>, StoreError> {
    let collections = get_endpoint_collections(path).await?;

    let mut not_started = Vec::new();
    for entry in collections {
        if entry.status != Status::NotStarted {
            continue;
        }

        not_started.push(entry);
    }

    Ok(not_started)
}

/**
 * Update `CollectionInfo` at central `collections.redb` file.
 */
pub(crate) async fn update_collection(
    info: &CollectionInfo,
    db: &Database,
) -> Result<(), StoreError> {
    let status = update_info_db(info, db);

    if status.is_err() {
        error!("[server] Could not update collection database");
        return Err(StoreError::WriteFile);
    }
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

/// Update `CollectionInfo` in database
fn update_info_db(info: &CollectionInfo, database: &Database) -> Result<(), Error> {
    let write_txn = database.begin_write()?;
    {
        let name: TableDefinition<'_, u64, String> = TableDefinition::new("collections");
        let mut table = write_txn.open_table(name)?;
        table.insert(info.id, serde_json::to_string(info).unwrap_or_default())?;
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
    use crate::filestore::collections::{
        add_endpoint_db, get_endpoint_collections, save_collection, update_collection,
        update_info_db, write_db,
    };
    use crate::utils::filesystem::create_dirs;
    use common::server::collections::{CollectionInfo, CollectionRequest, Status};
    use redb::Database;
    use std::collections::HashSet;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_save_collection() {
        create_dirs("./tmp/save").await.unwrap();
        let path = "./tmp/save/test.redb";

        let mut targets = HashSet::new();
        targets.insert(String::from("dafasdf"));

        let data = CollectionRequest {
            targets,
            collection:String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="), 
            targets_completed: HashSet::new(),
            info: CollectionInfo {
            id: 0,
            name: String::from("test"),
            created: 10,
            status: Status::NotStarted,
            duration: 0,
            start_time: 0,
        } };

        let db = Database::create(path).unwrap();

        save_collection(data, &db).await.unwrap();
    }

    #[tokio::test]
    async fn test_get_endpoint_collections() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/3482136c-3176-4272-9bd7-b79f025307d6");

        let result = get_endpoint_collections(&test_location.display().to_string())
            .await
            .unwrap();

        assert_eq!(result[0].id, 1);
        assert_eq!(result[0].name, "randomjob");
        assert_eq!(result[0].created, 10);
        assert_eq!(result[0].status, Status::NotStarted);
    }

    #[tokio::test]
    async fn test_update_collection() {
        let path = "./tmp/asdfasfd";

        let mut targets = HashSet::new();
        targets.insert(String::from("dafasdf"));

        let mut data = CollectionRequest {
            targets,
            collection:String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="), 
            targets_completed: HashSet::new(),
            info: CollectionInfo {
            id: 0,
            name: String::from("test"),
            created: 10,
            status: Status::NotStarted,
            duration: 0,
            start_time: 0,
        } };

        let db = Database::create(path).unwrap();

        save_collection(data.clone(), &db).await.unwrap();
        data.info.status = Status::Finished;
        update_collection(&data.info, &db).await.unwrap();
    }

    #[tokio::test]
    async fn test_write_db() {
        create_dirs("./tmp/save").await.unwrap();
        let path = "./tmp/save/collections1.redb";

        let mut targets = HashSet::new();
        targets.insert(String::from("dafasdf"));

        let data = CollectionRequest {
            targets,
            collection:String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="), 
            targets_completed: HashSet::new(),
            info: CollectionInfo {
            id: 0,
            name: String::from("test"),
            created: 10,
            status: Status::NotStarted,
            duration: 0,
            start_time: 0,
        } };

        let db = Database::create(path).unwrap();

        write_db(&data, &db).unwrap();
    }

    #[tokio::test]
    async fn test_update_info_db() {
        create_dirs("./tmp").await.unwrap();
        let path = "./tmp/save/test2.redb";

        let mut targets = HashSet::new();
        targets.insert(String::from("dafasdf"));

        let mut data = CollectionRequest {
            targets,
            collection:String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="), 
            targets_completed: HashSet::new(),
            info: CollectionInfo {
            id: 0,
            name: String::from("test"),
            created: 10,
            status: Status::NotStarted,
            duration: 0,
            start_time: 0,
        } };

        let db = Database::create(path).unwrap();

        save_collection(data.clone(), &db).await.unwrap();
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
            collection:String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="), 
            targets_completed: HashSet::new(),
            info: CollectionInfo {
            id: 0,
            name: String::from("test"),
            created: 10,
            status: Status::NotStarted,
            duration: 0,
            start_time: 0,
        } };

        let db = Database::create(path).unwrap();

        save_collection(data.clone(), &db).await.unwrap();
        add_endpoint_db("asdfasdfafsd", &data.info, &db).unwrap();
    }
}
