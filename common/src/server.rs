use crate::system::{Cpus, DiskDrives, Memory};
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

#[derive(Debug, Serialize, Deserialize)]
pub struct Pulse {
    pub endpoint_id: String,
    pub jobs_running: u32,
    pub pulse: bool,
    pub timestamp: u64,
    pub platform: String,
}
