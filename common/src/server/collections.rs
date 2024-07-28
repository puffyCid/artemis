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
    pub data: Value,
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
    pub targets_completed: HashSet<String>,
    pub info: CollectionInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CollectionInfo {
    /**Collection ID */
    pub id: u64,
    /**
     * Target Endpoint ID. This is not set on the initial request. It gets filled in on the response.  
     * Initial request can target more than one endpoint_id (See `CollectionRequest`)
     * When the target completes the collection, it fills in this struct
     * */
    pub endpoint_id: String,
    /**Name of collection */
    pub name: String,
    /**When Collection is created */
    pub created: u64,
    /**
     * Status of the collection
     * This is set to `NotStarted` when created. Updated to `Started` when target receives it
     * Target endpoint updates the status upon completion
     */
    pub status: Status,
    /**When endpoint should start Collection. This when the server sends the collection to the target */
    pub start_time: u64,
    /**When the target actually started the collection */
    pub started: u64,
    /**When target completed the collection */
    pub completed: u64,
    /**How long collection should run before stopping */
    pub timeout: u64,
    /**
     * Target platform. This is not set when creating the collection
     * Target endpoint fills it in when running
     */
    pub platform: Option<String>,
    /**
     * Target hostname. This is not set when creating the Collection
     * Target endpoint fills it in when running
     */
    pub hostname: Option<String>,
    /**How long the collection ran */
    pub duration: u64,
    /**Base64 Collection script */
    pub collection: String,
    /**Tags associated with the collectoin */
    pub tags: Vec<String>,
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

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub struct CollectionTargets {
    pub targets: Vec<String>,
    pub id: u64,
}
