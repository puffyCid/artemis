use super::error::CollectError;
use crate::{enrollment::enroll::bad_request, start::DaemonConfig};
use log::{error, info};
use reqwest::{StatusCode, blocking::Client};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub(crate) struct CollectResponse {
    /// Base64 toml endpoint collection
    pub(crate) collection: String,
    /// If invalid we should enroll again
    pub(crate) endpoint_invalid: bool,
    /// ID for the collection
    pub(crate) collection_id: u64,
}

#[derive(Serialize, Debug)]
pub(crate) struct CollectRequest {
    /// Unique endpoint ID that was provided from the server upon enrollment
    endpoint_id: String,
}

#[derive(Serialize, Debug, Copy, Clone)]
pub(crate) enum CollectionStatus {
    Complete,
    Error,
}

#[derive(Serialize, Debug)]
pub(crate) struct CompleteRequest {
    /// Status of the collection
    collection_status: CollectionStatus,
    /// Collection ID provided
    collection_id: u64,
}

#[derive(Deserialize, Debug)]
pub(crate) struct CompleteResponse {
    /// If invalid we should enroll again
    pub(crate) endpoint_invalid: bool,
}

pub(crate) trait CollectEndpoint {
    /// Check for any collection requests we need to run
    fn collect_request(&self) -> Result<CollectResponse, CollectError>;
    /// Send the status of our collection
    fn complete_collection(
        &self,
        status: CollectionStatus,
        collection_id: u64,
    ) -> Result<CompleteResponse, CollectError>;
}

impl CollectEndpoint for DaemonConfig {
    fn collect_request(&self) -> Result<CollectResponse, CollectError> {
        let url = format!(
            "{}:{}/v{}/{}",
            self.server.server.url,
            self.server.server.port,
            self.server.server.version,
            self.server.server.collections
        );

        let req = CollectRequest {
            endpoint_id: self.server.daemon.endpoint_id.clone(),
        };

        let client = Client::new();
        let mut builder = client.post(&url).json(&req);
        builder = builder.header("accept", "application/json");

        let res = match builder.send() {
            Ok(result) => result,
            Err(err) => {
                error!("[daemon] Failed to send request for collection: {err:?}");
                return Err(CollectError::FailedCollect);
            }
        };
        if res.status() == StatusCode::BAD_REQUEST {
            let message = bad_request(&res.bytes().unwrap_or_default());
            error!("[daemon] Collection request was bad: {}", message.message);
            return Err(CollectError::BadCollect);
        }

        if res.status() == StatusCode::NO_CONTENT {
            info!("[daemon] No collection content from server");
            return Err(CollectError::NoCollection);
        }

        if res.status() != StatusCode::OK {
            error!("[daemon] Got non-Ok collection response");
            return Err(CollectError::CollectNotOk);
        }

        let bytes = match res.bytes() {
            Ok(result) => result,
            Err(err) => {
                error!("[daemon] Failed to get collection bytes: {err:?}");
                return Err(CollectError::FailedCollect);
            }
        };

        let collect_toml: CollectResponse = match serde_json::from_slice(&bytes) {
            Ok(result) => result,
            Err(err) => {
                error!("[daemon] Failed to serialize collect response: {err:?}");
                return Err(CollectError::FailedCollect);
            }
        };

        Ok(collect_toml)
    }

    fn complete_collection(
        &self,
        status: CollectionStatus,
        collection_id: u64,
    ) -> Result<CompleteResponse, CollectError> {
        let url = format!(
            "{}:{}/v{}/{}/status",
            self.server.server.url,
            self.server.server.port,
            self.server.server.version,
            self.server.server.collections
        );

        let req = CompleteRequest {
            collection_status: status,
            collection_id,
        };

        let client = Client::new();
        let mut builder = client.post(&url).json(&req);
        builder = builder.header("accept", "application/json");
        builder = builder.header(
            "x-artemis-endpoint_id",
            self.server.daemon.endpoint_id.clone(),
        );

        let res = match builder.send() {
            Ok(result) => result,
            Err(err) => {
                error!("[daemon] Failed to send request for collection status: {err:?}");
                return Err(CollectError::FailedCollect);
            }
        };
        if res.status() == StatusCode::BAD_REQUEST {
            let message = bad_request(&res.bytes().unwrap_or_default());
            error!(
                "[daemon] Collection status request was bad: {}",
                message.message
            );
            return Err(CollectError::BadCollect);
        }

        if res.status() != StatusCode::OK {
            error!("[daemon] Got non-Ok collection status response");
            return Err(CollectError::CollectNotOk);
        }

        let bytes = match res.bytes() {
            Ok(result) => result,
            Err(err) => {
                error!("[daemon] Failed to get collection status bytes: {err:?}");
                return Err(CollectError::FailedCollect);
            }
        };

        let collect_toml: CompleteResponse = match serde_json::from_slice(&bytes) {
            Ok(result) => result,
            Err(err) => {
                error!("[daemon] Failed to serialize collect status response: {err:?}");
                return Err(CollectError::FailedCollect);
            }
        };

        Ok(collect_toml)
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        collection::collect::CollectEndpoint,
        start::DaemonConfig,
        utils::{config::server, encoding::base64_decode_standard},
    };
    use httpmock::{Method::POST, MockServer};
    use serde_json::json;
    use std::path::PathBuf;

    #[test]
    fn test_collect_request() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/configs/server.toml");

        let mock_server = MockServer::start();
        let port = mock_server.port();

        let mock_me = mock_server.mock(|when, then| {
            when.method(POST)
                .path("/v1/endpoint/collections")
                .body_contains("my important key");
            then.status(200)
                .header("content-type", "application/json")
                .json_body(json!({ "collection": "CltvdXRwdXRdCm5hbWUgPSAibGludXhfY29sbGVjdGlvbiIKZGlyZWN0b3J5ID0gIi4vdG1wIgpmb3JtYXQgPSAianNvbiIKY29tcHJlc3MgPSBmYWxzZQp0aW1lbGluZSA9IGZhbHNlCmVuZHBvaW50X2lkID0gImFiZGMiCmNvbGxlY3Rpb25faWQgPSAxCm91dHB1dCA9ICJsb2NhbCIKCltbYXJ0aWZhY3RzXV0KYXJ0aWZhY3RfbmFtZSA9ICJwcm9jZXNzZXMiClthcnRpZmFjdHMucHJvY2Vzc2VzXQptZDUgPSBmYWxzZQpzaGExID0gZmFsc2UKc2hhMjU2ID0gZmFsc2UKbWV0YWRhdGEgPSBmYWxzZQoKW1thcnRpZmFjdHNdXQphcnRpZmFjdF9uYW1lID0gInN5c3RlbWluZm8iCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAic2hlbGxfaGlzdG9yeSIKCltbYXJ0aWZhY3RzXV0KYXJ0aWZhY3RfbmFtZSA9ICJjaHJvbWl1bS1oaXN0b3J5IgoKW1thcnRpZmFjdHNdXQphcnRpZmFjdF9uYW1lID0gImNocm9taXVtLWRvd25sb2FkcyIKCltbYXJ0aWZhY3RzXV0KYXJ0aWZhY3RfbmFtZSA9ICJmaXJlZm94LWhpc3RvcnkiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAiZmlyZWZveC1kb3dubG9hZHMiCgpbW2FydGlmYWN0c11dCmFydGlmYWN0X25hbWUgPSAiY3JvbiI=", "endpoint_invalid": false, "collection_id": 1 }));
        });

        let server_config = server(test_location.to_str().unwrap(), Some("./tmp/artemis")).unwrap();
        let mut config = DaemonConfig {
            server: server_config,
        };
        config.server.server.port = port;

        let status = config.collect_request().unwrap();
        mock_me.assert();
        assert_eq!(status.endpoint_invalid, false);
        assert!(status.collection.len() > 100);

        let data = base64_decode_standard(&status.collection).unwrap();
        forensics::core::parse_toml_data(&data).unwrap();
    }
}
