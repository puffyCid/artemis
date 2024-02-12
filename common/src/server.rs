use std::collections::HashSet;

use crate::system::{Cpus, DiskDrives, LoadPerformance, Memory, Processes};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum EndpointOS {
    Windows,
    Darwin,
    Linux,
    All,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerInfo {
    pub memory_used: u64,
    pub total_memory: u64,
    pub cpu_usage: Vec<f32>,
    pub disk_info: Vec<DiskInfo>,
    pub uptime: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiskInfo {
    pub disk_usage: u64,
    pub disk_size: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EndpointRequest {
    pub pagination: String,
    pub filter: EndpointOS,
    pub tags: Vec<String>,
    pub search: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct EndpointList {
    pub os: String,
    pub hostname: String,
    pub version: String,
    pub id: String,
    pub last_heartbeat: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Heartbeat {
    pub endpoint_id: String,
    pub heartbeat: bool,
    pub jobs_running: u32,
    pub hostname: String,
    pub timestamp: u64,
    pub cpu: Vec<Cpus>,
    pub disks: Vec<DiskDrives>,
    pub memory: Memory,
    pub boot_time: u64,
    pub os_version: String,
    pub uptime: u64,
    pub kernel_version: String,
    pub platform: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Command {
    /**Unique list of endpoint IDs */
    pub targets: HashSet<String>,
    /**Job to send to the targets */
    pub job: JobInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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
    Script,
    Unknown,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProcessJob {
    pub metadata: JobMetadata,
    pub job: JobInfo,
    pub data: Vec<Processes>,
}
