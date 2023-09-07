use super::systeminfo::{Cpus, Disks, Memory};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Heartbeat {
    pub(crate) endpoint_id: String,
    pub(crate) heartbeat: bool,
    pub(crate) jobs_running: u32,
    pub(crate) hostname: String,
    pub(crate) timestamp: u64,
    pub(crate) cpu: Vec<Cpus>,
    pub(crate) disks: Vec<Disks>,
    pub(crate) memory: Memory,
    pub(crate) boot_time: u64,
    pub(crate) os_version: String,
    pub(crate) uptime: u64,
    pub(crate) kernel_version: String,
    pub(crate) platform: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub(crate) struct Pulse {
    pub(crate) endpoint_id: String,
    pub(crate) jobs_running: u32,
    pub(crate) pulse: bool,
    pub(crate) timestamp: u64,
}
