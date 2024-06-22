use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/**
 * Data collection request sent over websockets. Also called "QC"
 */
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct QuickCollection {
    /**What target endpoint to collect data from */
    pub target: String,
    /**Type of data to collect. Can never be artifact type */
    pub collection_type: CollectionType,
}

/**
 * Data sent back over websockets
 */
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct QuickResponse {
    /**Endpoint ID */
    pub id: String,
    /** The type of data returned from the collection */
    pub collection_type: CollectionType,
    pub platform: String,
    /**The data returned */
    pub data: Vec<Value>,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum CollectionType {
    Processes,
    Filelist,
}

/**
 * This collection is for complex and verbose data. The request is sent over websockets but the data will be uploaded using POST requets
 */
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CollectionRequest {
    /**Endpoint target */
    pub targets: HashSet<String>,
    /**Base64 encoded TOML collection */
    pub collection: String,
    pub targets_completed: HashSet<String>,
    pub info: CollectionInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CollectionInfo {
    /**Collection ID */
    pub id: u64,
    pub name: String,
    /**When Collection is created */
    pub created: u64,
    pub status: Status,
    /**When endpoint should start Collection */
    pub start_time: u64,
    /**How long Collection should run */
    pub duration: u64,
}

/**
 * This collection is for complex and verbose data. The response uploaded via POST requests
 */
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CollectionResponse {
    /**Endpoint target */
    pub target: String,
    pub info: CollectionInfo,
    /**When endpoint started the collection */
    pub started: u64,
    /**When endpoint finished the collection */
    pub finished: u64,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Status {
    /**Has not be sent to the target endpoint */
    NotStarted,
    /**Collection request has been sent to the target */
    Started,
    Finished,
    Failed,
    Cancelled,
}
