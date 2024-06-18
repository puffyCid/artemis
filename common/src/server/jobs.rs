use crate::system::{LoadPerformance, Processes};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Command {
    /**Unique list of endpoint IDs */
    pub targets: Vec<String>,
    /**Job to send to the targets */
    pub job: JobInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct JobInfo {
    pub id: u64,
    pub name: String,
    /**When Job is created */
    pub created: u64,
    /**When endpoint executes the Job */
    pub started: u64,
    /**When endpoint finishes the Job */
    pub finished: u64,
    pub status: Status,
    /**Base64 encoded TOML collection */
    pub collection: String,
    /**When endpoint should start job */
    pub start_time: u64,
    /**How long job should run */
    pub duration: u64,
    pub action: Action,
    pub job_type: JobType,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Status {
    NotStarted,
    Started,
    Finished,
    Failed,
    Cancelled,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum Action {
    Start,
    Stop,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone)]
pub enum JobType {
    Collection,
    Processes,
    Filelist,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct JobMetadata {
    pub endpoint_id: String,
    pub uuid: String,
    pub id: u64,
    pub artifact_name: String,
    pub complete_time: u64,
    pub start_time: u64,
    pub hostname: String,
    pub os_version: String,
    pub platform: String,
    pub kernel_version: String,
    pub load_performance: LoadPerformance,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct ProcessJob {
    pub metadata: JobMetadata,
    pub job: JobInfo,
    pub data: Vec<Processes>,
}
