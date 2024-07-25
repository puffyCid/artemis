use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum EndpointOS {
    Windows,
    MacOS,
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

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct EndpointRequest {
    pub offset: i32,
    pub filter: EndpointOS,
    pub tags: Vec<String>,
    pub search: String,
    pub count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct CollectRequest {
    pub offset: i32,
    pub tags: Vec<String>,
    pub search: String,
    pub count: i32,
}

#[derive(Debug, Serialize, Deserialize, Clone, Ord, PartialEq, Eq, PartialOrd)]
pub struct EndpointList {
    pub os: String,
    pub hostname: String,
    pub version: String,
    pub id: String,
    pub last_heartbeat: u64,
    pub ip: String,
    pub artemis_version: String,
}
