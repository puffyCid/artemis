use common::server::enrollment::Enrollment;
use common::system::{Cpus, DiskDrives, Memory};
use sysinfo::{Disks, System};

/// Collect system info for enrollment request
pub(crate) fn gather_info() -> Enrollment {
    let mut system = System::new();
    Enrollment {
        boot_time: System::boot_time(),
        hostname: System::host_name().unwrap_or_default(),
        os_version: System::os_version().unwrap_or_default(),
        uptime: System::uptime(),
        kernel_version: System::kernel_version().unwrap_or_default(),
        platform: System::name().unwrap_or_default(),
        cpu: get_cpu(&mut system),
        disks: get_disks(),
        memory: get_memory(&mut system),
        ip: String::from("IP: TODO"),
        artemis_version: env!("CARGO_PKG_VERSION").to_string(),
    }
}

/// Get Disk info from system
fn get_disks() -> Vec<DiskDrives> {
    let mut disks = Disks::new_with_refreshed_list();

    let mut disk_vec = Vec::new();
    for disk in &mut disks {
        let fs_type = disk.file_system().to_str().unwrap_or_default();
        let disk_data = DiskDrives {
            disk_type: format!("{:?}", disk.kind()),
            file_system: fs_type.to_string(),
            mount_point: disk.mount_point().display().to_string(),
            total_space: disk.total_space(),
            available_space: disk.available_space(),
            removable: disk.is_removable(),
        };
        disk_vec.push(disk_data);
    }
    disk_vec
}

/// Get CPU info from system
fn get_cpu(system: &mut System) -> Vec<Cpus> {
    system.refresh_cpu_all();
    let mut cpu_vec: Vec<Cpus> = Vec::new();

    for cpu in system.cpus() {
        let cpu = Cpus {
            frequency: cpu.frequency(),
            cpu_usage: cpu.cpu_usage(),
            name: cpu.name().to_string(),
            vendor_id: cpu.vendor_id().to_string(),
            brand: cpu.brand().to_string(),
            physical_core_count: system.physical_core_count().unwrap_or(0),
        };
        cpu_vec.push(cpu);
    }
    cpu_vec
}

/// Get Memory info from system
fn get_memory(system: &mut System) -> Memory {
    system.refresh_memory();
    Memory {
        available_memory: system.available_memory(),
        free_memory: system.free_memory(),
        free_swap: system.free_swap(),
        total_memory: system.total_memory(),
        total_swap: system.total_swap(),
        used_memory: system.used_memory(),
        used_swap: system.used_swap(),
    }
}

#[cfg(test)]
mod tests {
    use crate::enrollment::info::gather_info;
    use crate::enrollment::info::get_cpu;
    use crate::enrollment::info::get_disks;
    use crate::enrollment::info::get_memory;
    use sysinfo::System;

    #[test]
    fn test_gather_info() {
        let system_info = gather_info();
        assert_eq!(system_info.platform.is_empty(), false);
        assert!(system_info.cpu.len() > 1);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_macos_disks() {
        let system_info = get_disks();
        assert_eq!(system_info.len(), 2);
        assert_eq!(system_info[0].disk_type.is_empty(), false);
        assert_eq!(system_info[1].disk_type.is_empty(), false);

        assert_eq!(system_info[0].mount_point, "/");
        assert_eq!(system_info[1].mount_point, "/System/Volumes/Data");

        assert_eq!(system_info[0].removable, false);
        assert_eq!(system_info[1].removable, false);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_windows_disks() {
        let system_info = get_disks();
        assert!(system_info.len() >= 1);
        assert_eq!(system_info[0].disk_type.is_empty(), false);
        assert_eq!(system_info[0].mount_point.contains(":\\"), true);
        assert_eq!(system_info[0].removable, false);
    }

    #[test]
    #[cfg(target_os = "linux")]
    fn test_get_linux_disks() {
        let system_info = get_disks();
        assert!(system_info.len() >= 1);
    }

    #[test]
    fn test_get_cpu() {
        let mut system = System::new();

        let system_info = get_cpu(&mut system);
        assert!(system_info.len() > 1);
    }

    #[test]
    fn test_get_memory() {
        let mut system = System::new();

        let system_info = get_memory(&mut system);
        assert!(system_info.available_memory > 1);
        assert!(system_info.free_memory > 1);
        assert!(system_info.total_memory > 1);
        assert!(system_info.used_memory > 1);
    }
}
