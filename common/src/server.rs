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
    pub last_pulse: u64,
}
