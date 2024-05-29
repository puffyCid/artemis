use crate::system::{Cpus, DiskDrives, Memory};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct EnrollmentResponse {
    pub endpoint_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Enrollment {
    pub boot_time: u64,
    pub hostname: String,
    pub ip: String,
    pub os_version: String,
    pub uptime: u64,
    pub kernel_version: String,
    pub platform: String,
    pub cpu: Vec<Cpus>,
    pub disks: Vec<DiskDrives>,
    pub memory: Memory,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct EnrollSystem {
    pub enroll_key: String,
    pub enrollment_info: Enrollment,
}
