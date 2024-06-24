use crate::socket::error::SocketError;
use artemis_core::artifacts::os::processes::process::proc_list;
use common::{
    files::Hashes,
    server::collections::{QuickCollection, QuickResponse},
};
use sysinfo::System;

/// Get processes listing for system from artemis core
pub(crate) async fn collect_processes(
    quick: &QuickCollection,
) -> Result<QuickResponse, SocketError> {
    let hashes = Hashes {
        md5: false,
        sha1: false,
        sha256: false,
    };
    let procs_results = proc_list(&hashes, false);
    let procs = match procs_results {
        Ok(results) => results,
        Err(_err) => return Err(SocketError::QuickCollection),
    };

    let res = QuickResponse {
        id: quick.target.clone(),
        collection_type: quick.collection_type.clone(),
        platform: System::name().unwrap_or_default(),
        data: serde_json::to_value(&procs).unwrap(),
    };

    Ok(res)
}

#[cfg(test)]
mod tests {
    use crate::socket::actions::processes::collect_processes;
    use common::server::collections::{CollectionType, QuickCollection};

    #[tokio::test]
    async fn test_collect_processes() {
        let quick = QuickCollection {
            target: String::from("asfdasdf"),
            collection_type: CollectionType::Processes,
        };

        let results = collect_processes(&quick).await.unwrap();
        assert!(results.data.is_array());
    }
}
