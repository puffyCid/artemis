use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Serialize)]
pub struct SystemInfo {
    pub boot_time: String,
    pub hostname: String,
    pub os_version: String,
    pub uptime: u64,
    pub kernel_version: String,
    pub platform: String,
    pub cpu: Vec<Cpus>,
    pub disks: Vec<DiskDrives>,
    pub memory: Memory,
    pub interfaces: Vec<NetworkInterface>,
    pub performance: LoadPerformance,
    pub version: String,
    pub rust_version: String,
    pub build_date: String,
    pub product_name: String,
    pub product_family: String,
    pub product_serial: String,
    pub product_uuid: String,
    pub product_version: String,
    pub vendor: String,
}

#[derive(Debug, Serialize)]
pub struct SystemInfoMetadata {
    pub hostname: String,
    pub os_version: String,
    pub kernel_version: String,
    pub platform: String,
    pub performance: LoadPerformance,
    pub interfaces: Vec<NetworkInterface>,
    pub version: String,
    pub rust_version: String,
    pub build_date: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Cpus {
    pub frequency: u64,
    pub cpu_usage: f32,
    pub name: String,
    pub vendor_id: String,
    pub brand: String,
    pub physical_core_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DiskDrives {
    pub disk_type: String,
    pub file_system: String,
    pub mount_point: String,
    pub total_space: u64,
    pub available_space: u64,
    pub removable: bool,
    pub name: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Memory {
    pub available_memory: u64,
    pub free_memory: u64,
    pub free_swap: u64,
    pub total_memory: u64,
    pub total_swap: u64,
    pub used_memory: u64,
    pub used_swap: u64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct LoadPerformance {
    pub avg_one_min: f64,
    pub avg_five_min: f64,
    pub avg_fifteen_min: f64,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Processes {
    pub full_path: String,
    pub name: String,
    pub path: String,
    pub pid: u32,
    pub ppid: u32,
    pub environment: String,
    pub status: String,
    pub arguments: String,
    pub memory_usage: u64,
    pub virtual_memory_usage: u64,
    pub start_time: String,
    pub uid: String,
    pub gid: String,
    pub md5: String,
    pub sha1: String,
    pub sha256: String,
    pub binary_info: Value,
}

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]

pub struct NetworkInterface {
    pub ip: String,
    pub mac: String,
    pub name: String,
}
