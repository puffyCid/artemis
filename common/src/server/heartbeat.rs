use crate::system::{Cpus, DiskDrives, Memory};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Heartbeat {
    pub endpoint_id: String,
    pub heartbeat: bool,
    pub jobs_running: u32,
    pub hostname: String,
    pub ip: String,
    pub timestamp: u64,
    pub cpu: Vec<Cpus>,
    pub disks: Vec<DiskDrives>,
    pub memory: Memory,
    pub boot_time: u64,
    pub os_version: String,
    pub uptime: u64,
    pub kernel_version: String,
    pub platform: String,
    pub artemis_version: String,
}
