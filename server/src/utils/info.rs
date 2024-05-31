use common::server::webui::{DiskInfo, ServerInfo};
use sysinfo::{Disks, System};

/// Get server system info
pub(crate) fn server_info() -> ServerInfo {
    let mut system = System::new();
    system.refresh_memory();
    system.refresh_cpu_usage();

    let mut disks = Disks::new_with_refreshed_list();
    let mut cpus = Vec::new();
    let mut disks_data = Vec::new();

    for cpu in system.cpus() {
        cpus.push(cpu.cpu_usage());
    }

    for disk in &mut disks {
        let disk_info = DiskInfo {
            disk_usage: disk.total_space() - disk.available_space(),
            disk_size: disk.total_space(),
        };
        disks_data.push(disk_info);
    }

    ServerInfo {
        memory_used: system.used_memory(),
        total_memory: system.total_memory(),
        cpu_usage: cpus,
        disk_info: disks_data,
        uptime: System::uptime(),
    }
}

#[cfg(test)]
mod tests {
    use super::server_info;

    #[test]
    fn test_server_info() {
        let results = server_info();
        assert!(results.memory_used > 0);
    }
}
