use crate::{
    filestore::{
        collections::{collection_status, set_collection_info},
        database::update_info_db,
    },
    server::ServerState,
    utils::{
        filesystem::{create_dirs, write_file},
        uuid::generate_uuid,
    },
};
use axum::{
    extract::{Multipart, State},
    http::StatusCode,
};
use common::server::collections::{CollectionInfo, Status};
use log::{error, warn};
use redb::Database;

/// Process uploaded data
pub(crate) async fn upload_collection(
    State(state): State<ServerState>,
    mut multipart: Multipart,
) -> Result<(), StatusCode> {
    let path = state.config.endpoint_server.storage;

    let mut endpoint_id = String::new();
    let mut platform = String::new();

    while let Some(field) = multipart.next_field().await.unwrap_or(None) {
        let name = field.name().unwrap_or_default().to_string();

        if name == "collection-info" {
            let data = field.text().await.unwrap_or_default();
            let serde_result = serde_json::from_str(&data);
            let serde_value: CollectionInfo = match serde_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[server] Failed to deserialzie collection upload metadata: {err:?}");
                    return Err(StatusCode::BAD_REQUEST);
                }
            };
            update_collection_status(&path, &serde_value, &state.central_collect_db).await?;
            if serde_value.platform.is_none() || serde_value.hostname.is_none() {
                error!("[server] Did not receive all required info in response");
                return Err(StatusCode::BAD_REQUEST);
            }
            endpoint_id = serde_value.endpoint_id.clone();
            platform = serde_value.platform.clone().unwrap_or_default();
            let path = format!("{path}/{platform}/{endpoint_id}");
            let status_result = collection_status(&path, &serde_value.id).await;
            let status = match status_result {
                Ok(result) => result,
                Err(err) => {
                    error!("[server] Failed to check endpoint status for upload: {err:?}");
                    return Err(StatusCode::BAD_REQUEST);
                }
            };
            if status != Status::Started {
                warn!("[server] Received uploaded data for endpoint but status is unexpected");
                return Err(StatusCode::BAD_REQUEST);
            }
            let _ = set_collection_info(&path, &[serde_value.id], &serde_value).await;
        } else if name == "collection" {
            let filename_option = field.file_name();
            let filename = if let Some(result) = filename_option {
                result.to_string()
            } else {
                warn!("[server] Filename not provided in upload. Generated a random one!");
                format!("{}.jsonl.gz", generate_uuid())
            };

            let data = field.bytes().await.unwrap_or_default();
            let endpoint_dir = format!("{path}/{platform}/{endpoint_id}");
            write_collection(&endpoint_dir, &filename, &data).await?;
        }
    }
    Ok(())
}

/// Update the Collection DB using the uploaded collection-info data
async fn update_collection_status(
    path: &str,
    collect: &CollectionInfo,
    db: &Database,
) -> Result<(), StatusCode> {
    if path.is_empty() {
        error!("[server] No endpoint path provided cannot update collections.redb");
        return Err(StatusCode::BAD_REQUEST);
    }

    let status = update_info_db(collect, db);
    if status.is_err() {
        error!(
            "[server] Could not update collection info for {path}: {:?}",
            status.unwrap_err()
        );
        return Err(StatusCode::BAD_REQUEST);
    }

    Ok(())
}

/// Write data to endpoint storage directory
async fn write_collection(
    endpoint_dir: &str,
    filename: &str,
    data: &[u8],
) -> Result<(), StatusCode> {
    // Endpoint storage directory should have been created upon enrollment. But check in case
    let collections = format!("{endpoint_dir}/collections");
    let status = create_dirs(&collections).await;
    if status.is_err() {
        error!(
            "[server] Could not create {collections} storage directory: {:?}",
            status.unwrap_err()
        );
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Only decompress data smaller than 2GB
    let max_size = 2147483648;
    if data.len() < max_size {
        let decom_name = filename.trim_end_matches(".gz");
        let endpoint_path = format!("{collections}/{decom_name}");
        // Write the data to endpoint directory,  but decompress first
        let status = write_file(data, &endpoint_path, true).await;
        if status.is_err() {
            error!(
                "[server] Could not write data to {endpoint_path} storage directory: {:?}",
                status.unwrap_err()
            );
        } else {
            return Ok(());
        }

        warn!("[server] Could not decompress and write data to {collections}. Trying compressed data!");
    }

    let endpoint_path = format!("{collections}/{filename}");

    // Write the compressed data to endpoint directory
    let status = write_file(data, &endpoint_path, false).await;
    if status.is_err() {
        error!(
            "[server] Could not write data to {endpoint_path} storage directory: {:?}",
            status.unwrap_err()
        );
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::filestore::database::save_collection;
    use crate::uploads::upload::write_collection;
    use crate::utils::filesystem::create_dirs;
    use crate::{
        uploads::upload::update_collection_status,
        utils::{config::read_config, uuid::generate_uuid},
    };
    use common::server::collections::{CollectionInfo, CollectionRequest, Status};
    use redb::Database;
    use std::collections::HashSet;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_update_collection_status() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");
        create_dirs("./tmp/uploads").await.unwrap();

        let mut value =
           CollectionInfo {
                id: 10,
                endpoint_id: String::from("dafasdf"),
                name: String::from("dasfasdfsa"),
                created: 10,
                status: Status::Started,
                start_time: 0,
                duration: 10,
                tags: Vec::new(),
                collection: String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="), 
                started: 1000,
                completed: 2000,
                timeout: 1000,
                platform: Some(String::from("Darwin")),
                hostname: Some(String::from("cxvasdf")),
        };

        let mut targets = HashSet::new();
        targets.insert(String::from("dafasdf"));
        let req = CollectionRequest {
            targets,
            targets_completed: HashSet::new(),
            info: value.clone(),
        };

        let db = Database::create("./tmp/uploads/test2.redb").unwrap();

        save_collection(req, &db, "./tmp/uploads").await.unwrap();

        value.status = Status::Finished;

        update_collection_status("./tmp/uploads", &value, &db)
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_write_collection() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/server.toml");

        let config = read_config(&test_location.display().to_string())
            .await
            .unwrap();
        let endpoint_id = generate_uuid();

        let path = format!("{}/{endpoint_id}", config.endpoint_server.storage);
        let filename = "test.jsonl.gz";
        let data = [
            31, 139, 8, 0, 89, 135, 7, 101, 0, 255, 5, 128, 177, 9, 0, 32, 16, 3, 87, 209, 27, 195,
            121, 20, 44, 2, 129, 111, 190, 16, 119, 15, 143, 123, 36, 179, 6, 237, 210, 158, 252,
            0, 132, 255, 53, 22, 19, 0, 0, 0,
        ];

        write_collection(&path, filename, &data).await.unwrap();
    }
}
