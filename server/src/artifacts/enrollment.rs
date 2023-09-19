use super::systeminfo::{Cpus, Disks, Memory};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub(crate) struct EndpointInfo {
    pub(crate) boot_time: u64,
    pub(crate) hostname: String,
    pub(crate) os_version: String,
    pub(crate) uptime: u64,
    pub(crate) kernel_version: String,
    pub(crate) platform: String,
    pub(crate) cpu: Vec<Cpus>,
    pub(crate) disks: Vec<Disks>,
    pub(crate) memory: Memory,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Endpoint {
    pub(crate) endpoint_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
/// Static data about and endpoint that rarely changes
pub(crate) struct EndpointStatic {
    pub(crate) hostname: String,
    pub(crate) platform: String,
    pub(crate) tags: Vec<String>,
    pub(crate) notes: Vec<Notes>,
    pub(crate) checkin: u64,
    pub(crate) id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct Notes {
    comment: String,
    author: String,
    created: u64,
}
