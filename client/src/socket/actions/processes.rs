use crate::socket::error::SocketError;
use common::server::collections::{QuickCollection, QuickResponse};
use serde_json::Value;
use sysinfo::System;

/// Get processes listing for system
pub(crate) async fn collect_processes(
    quick: &QuickCollection,
) -> Result<QuickResponse, SocketError> {
    /*TODO: Add proc listing via sysinfo crate */

    let res = QuickResponse {
        id: quick.target.clone(),
        collection_type: quick.collection_type.clone(),
        platform: System::name().unwrap_or_default(),
        data: Value::Null,
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
        assert!(results.data.is_null());
    }
}
