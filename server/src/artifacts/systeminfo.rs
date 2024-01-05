use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct SystemInfo {
    pub(crate) boot_time: u64,
    pub(crate) hostname: String,
    pub(crate) os_version: String,
    pub(crate) uptime: u64,
    pub(crate) kernel_version: String,
    pub(crate) platform: String,
    pub(crate) cpu: Vec<Cpus>,
    pub(crate) disks: Vec<Disks>,
    pub(crate) memory: Memory,
    pub(crate) performance: LoadPerformance,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Cpus {
    pub(crate) frequency: u64,
    pub(crate) cpu_usage: f32,
    pub(crate) name: String,
    pub(crate) vendor_id: String,
    pub(crate) brand: String,
    pub(crate) physical_core_count: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Disks {
    pub(crate) disk_type: String,
    pub(crate) file_system: String,
    pub(crate) mount_point: String,
    pub(crate) total_space: u64,
    pub(crate) available_space: u64,
    pub(crate) removable: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub(crate) struct Memory {
    pub(crate) available_memory: u64,
    pub(crate) free_memory: u64,
    pub(crate) free_swap: u64,
    pub(crate) total_memory: u64,
    pub(crate) total_swap: u64,
    pub(crate) used_memory: u64,
    pub(crate) used_swap: u64,
}

#[derive(Debug, Deserialize)]
pub(crate) struct LoadPerformance {
    pub(crate) avg_one_min: f64,
    pub(crate) avg_five_min: f64,
    pub(crate) avg_fifteen_min: f64,
}
