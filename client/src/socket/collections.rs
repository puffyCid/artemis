use super::error::SocketError;
use crate::{
    filesystem::files::{append_file, is_file, write_file},
    socket::actions::processes::collect_processes,
};
use common::server::collections::{
    CollectionRequest, CollectionType, QuickCollection, QuickResponse,
};
use log::error;

/// Save collections to a file
pub(crate) async fn save_collection(
    collect: &CollectionRequest,
    storage_path: &str,
) -> Result<(), SocketError> {
    let job_path = format!("{storage_path}/collections.jsonl");
    let bytes_result = serde_json::to_vec(&collect);
    let mut bytes = match bytes_result {
        Ok(result) => result,
        Err(err) => {
            error!("[client] Could not serialize collection to bytes: {err:?}");
            return Err(SocketError::SaveCollection);
        }
    };

    bytes.push(b'\n');

    if !is_file(&job_path) {
        let status = write_file(&bytes, &job_path).await;
        if status.is_err() {
            error!(
                "[client] Could not write job to file: {:?}",
                status.unwrap_err()
            );
            return Err(SocketError::SaveCollection);
        }

        return Ok(());
    }

    let _ = append_file(&bytes, &job_path).await;

    Ok(())
}

/// Parse quick collection requests from server
pub(crate) async fn quick_collection(
    quick: &QuickCollection,
) -> Result<QuickResponse, SocketError> {
    match quick.collection_type {
        CollectionType::Processes => collect_processes(quick).await,
        CollectionType::Filelist => Err(SocketError::QuickCollection),
    }
}

#[cfg(test)]
mod tests {
    use crate::filesystem::directory::create_dirs;
    use crate::socket::collections::{quick_collection, save_collection};
    use common::server::collections::{
        CollectionInfo, CollectionRequest, CollectionType, QuickCollection, Status,
    };
    use std::collections::HashSet;

    #[tokio::test]
    async fn test_save_collection() {
        create_dirs("./tmp").await.unwrap();
        let mut targets = HashSet::new();
        targets.insert(String::from("dafasdf"));

        let data = CollectionRequest {
            targets,
            targets_completed: HashSet::new(),
            info: CollectionInfo {
                endpoint_id: Some(String::from("dafasdf")),
                id: 0,
                name: String::from("test"),
                created: 10,
                status: Status::NotStarted,
                duration: 0,
                start_time: 0,
                tags: Vec::new(),
                collection: String::from("c3lzdGVtID0gIndpbmRvd3MiCgpbb3V0cHV0XQpuYW1lID0gInByZWZldGNoX2NvbGxlY3Rpb24iCmRpcmVjdG9yeSA9ICIuL3RtcCIKZm9ybWF0ID0gImpzb24iCmNvbXByZXNzID0gZmFsc2UKZW5kcG9pbnRfaWQgPSAiNmM1MWIxMjMtMTUyMi00NTcyLTlmMmEtMGJkNWFiZDgxYjgyIgpjb2xsZWN0aW9uX2lkID0gMQpvdXRwdXQgPSAibG9jYWwiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAicHJlZmV0Y2giClthcnRpZmFjdHMucHJlZmV0Y2hdCmFsdF9kcml2ZSA9ICdDJwo="), 
        } };
        save_collection(&data, "./tmp/").await.unwrap();
    }

    #[tokio::test]
    async fn test_quick_collection() {
        create_dirs("./tmp").await.unwrap();
        let mut targets = HashSet::new();
        targets.insert(String::from("dafasdf"));

        let data = QuickCollection {
            collection_type: CollectionType::Processes,
            target: String::from("dafdasdfa"),
        };
        let results = quick_collection(&data).await.unwrap();
        assert!(results.data.is_array());
    }
}
