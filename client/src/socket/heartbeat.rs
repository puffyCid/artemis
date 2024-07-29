use crate::enrollment::info::gather_info;
use common::server::heartbeat::Heartbeat;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Generate heartbeat data to send to server
pub(crate) fn generate_heartbeat(id: &str) -> Heartbeat {
    let info = gather_info();

    Heartbeat {
        endpoint_id: id.to_string(),
        jobs_running: 0,
        hostname: info.hostname,
        ip: info.ip,
        timestamp: time_now(),
        cpu: info.cpu,
        disks: info.disks,
        memory: info.memory,
        boot_time: info.boot_time,
        os_version: info.os_version,
        uptime: info.uptime,
        kernel_version: info.kernel_version,
        platform: info.platform,
        artemis_version: info.artemis_version,
    }
}

/// Return time now in seconds or 0
pub(crate) fn time_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::new(0, 0))
        .as_secs()
}

#[cfg(test)]
mod tests {
    use crate::socket::heartbeat::{generate_heartbeat, time_now};

    #[test]
    fn test_generate_heartbeat() {
        let result = generate_heartbeat("test");
        assert!(!result.hostname.is_empty());
    }

    #[test]
    fn test_time_now() {
        assert!(time_now() != 0);
    }
}
