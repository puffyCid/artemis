use super::{endpoints::glob_paths, error::StoreError};
use crate::utils::{
    filesystem::{append_file, is_file, read_lines},
    time::time_now,
};
use common::server::collections::{CollectionInfo, CollectionRequest, Status};
use log::error;
use redb::{Database, Error, TableDefinition};
use tokio::fs::{remove_file, rename};

/// Save collection info associated with endpoint
pub(crate) async fn save_endpoint_collection(collection: &mut CollectionRequest, path: &str) {
    for target in &collection.targets {
        let paths = glob_paths(&format!("{path}/*/{target}/collections.jsonl"));
        if paths.is_err() {
            continue;
        }
        for path in paths.unwrap() {
            collection.info.endpoint_id = target.clone();
            let serde_result = serde_json::to_string(&collection.info);
            let value = match serde_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[server] Failed to serialize endpoint collection data: {err:?}");
                    continue;
                }
            };

            let limit = 1024 * 1024 * 1024 * 5;

            let status = append_file(&value, &path.full_path, &limit).await;
            if status.is_err() {
                error!("[server] Could not write endpoint collection file");
                continue;
            }
        }
    }
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
        if entry.status != Status::NotStarted || entry.start_time > time_now() {
            continue;
        }

        not_started.push(entry);
    }

    Ok(not_started)
}

/// Set collection IDs to specified status for endpoint
pub(crate) async fn set_collection_status(
    path: &str,
    ids: &[u64],
    status: &Status,
) -> Result<(), StoreError> {
    let mut collections = get_endpoint_collections(path).await?;
    let temp_file = format!("{path}/collections_temp.jsonl");
    let limit = 1024 * 1024 * 1024 * 5;

    for entry in &mut collections {
        let status = if !ids.contains(&entry.id) {
            append_file(
                &serde_json::to_string(entry).unwrap_or_default(),
                &temp_file,
                &limit,
            )
            .await
        } else {
            entry.status = status.clone();
            append_file(
                &serde_json::to_string(entry).unwrap_or_default(),
                &temp_file,
                &limit,
            )
            .await
        };

        if status.is_err() {
            error!(
                "[server] Could not write updated collections temp file: {:?}",
                status.unwrap_err()
            );
        }
    }

    let status = rename(&temp_file, &format!("{path}/collections.jsonl")).await;
    if status.is_err() {
        error!(
            "[server] Could not move collections temp file: {:?}",
            status.unwrap_err()
        );
    }

    let status = remove_file(&temp_file).await;
    if status.is_err() {
        error!(
            "[server] Could not delete collections temp file: {:?}",
            status.unwrap_err()
        );
    }
    Ok(())
}

/// Get collection status for endpoint. Path is full path to endpoint **including** the endpoint ID
pub(crate) async fn collection_status(path: &str, id: &u64) -> Result<Status, StoreError> {
    Ok(get_collection_info(path, id).await?.status)
}

/// Get collection info for endpoint. Path is full path to endpoint **including** the endpoint ID
pub(crate) async fn get_collection_info(
    path: &str,
    id: &u64,
) -> Result<CollectionInfo, StoreError> {
    let collections = get_endpoint_collections(path).await?;
    for ids in collections {
        if &ids.id == id {
            return Ok(ids);
        }
    }
    Err(StoreError::NoCollection)
}

/// Get the Collection script from the REDB database
fn get_collection_script(id: &u64, db: &Database) -> Result<String, Error> {
    let read_txn = db.begin_read()?;
    let name: TableDefinition<'_, u64, String> = TableDefinition::new("collections");

    let read_table = read_txn.open_table(name)?;
    let value = read_table.get(id)?;
    if let Some(entry) = value {
        let collect_value = serde_json::from_str(&entry.value());
        let serde_data: CollectionRequest = match collect_value {
            Ok(result) => result,
            Err(err) => {
                error!("[server] Could not deserialize collection data: {err:?}");
                return Err(Error::Corrupted(format!(
                    "Could not deserialize collection data: {err:?}"
                )));
            }
        };

        return Ok(serde_data.info.collection);
    }

    Err(Error::TableDoesNotExist(String::from("collections")))
}

#[cfg(test)]
mod tests {
    use crate::filestore::collections::{
        collection_status, get_endpoint_collections, set_collection_status,
    };
    use common::server::collections::Status;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_collection_status() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/3482136c-3176-4272-9bd7-b79f025307d6");

        let status = collection_status(test_location.to_str().unwrap(), &1)
            .await
            .unwrap();
        assert_eq!(status, Status::NotStarted)
    }

    #[tokio::test]
    async fn test_set_collection_status() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/3482136c-3176-4272-9bd7-b79f025307d6");

        set_collection_status(test_location.to_str().unwrap(), &[1], &Status::NotStarted)
            .await
            .unwrap();
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
}
