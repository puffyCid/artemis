use common::system::{Cpus, DiskDrives, LoadPerformance, Memory, SystemInfo};
use sysinfo::{Disks, System};

use crate::utils::time::unixepoch_to_iso;

/// Get Disk, CPU, Memory, and Performance info from system
pub(crate) fn get_info() -> SystemInfo {
    let mut system = System::new();
    SystemInfo {
        boot_time: unixepoch_to_iso(&(sysinfo::System::boot_time() as i64)),
        hostname: sysinfo::System::host_name().unwrap_or_else(|| String::from("Unknown hostname")),
        os_version: sysinfo::System::os_version()
            .unwrap_or_else(|| String::from("Unknown OS version")),
        uptime: sysinfo::System::uptime(),
        kernel_version: sysinfo::System::kernel_version()
            .unwrap_or_else(|| String::from("Unknown kernel version")),
        platform: sysinfo::System::name().unwrap_or_else(|| String::from("Unknown system name")),
        cpu: get_cpu(&mut system),
        disks: get_disks(),
        memory: get_memory(&mut system),
        performance: get_performance(),
    }
}

/// Get endpoint platform type. Supports more options than `get_platform_enum`
pub(crate) fn get_platform() -> String {
    sysinfo::System::name().unwrap_or_else(|| String::from("Unknown system name"))
}

#[derive(PartialEq, Debug)]
pub(crate) enum PlatformType {
    Linux,
    Macos,
    Windows,
    Unknown,
}
/// Get endpoint platform type enum. Use `get_platform` if you want a string.
pub(crate) fn get_platform_enum() -> PlatformType {
    let plat =
        sysinfo::System::long_os_version().unwrap_or_else(|| String::from("Unknown system name"));
    if plat.to_lowercase().contains("windows") {
        return PlatformType::Windows;
    } else if plat.to_lowercase().contains("macos") {
        return PlatformType::Macos;
    } else if plat.to_lowercase().contains("linux") {
        return PlatformType::Linux;
    }

    PlatformType::Unknown
}

/// Get Disk info from system
pub(crate) fn get_disks() -> Vec<DiskDrives> {
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
pub(crate) fn get_cpu(system: &mut System) -> Vec<Cpus> {
    system.refresh_cpu_all();
    let mut cpu_vec: Vec<Cpus> = Vec::new();

    for cpu in system.cpus() {
        let cpu = Cpus {
            frequency: cpu.frequency(),
            cpu_usage: cpu.cpu_usage(),
            name: cpu.name().to_string(),
            vendor_id: cpu.vendor_id().to_string(),
            brand: cpu.brand().to_string(),
            physical_core_count: System::physical_core_count().unwrap_or(0),
        };
        cpu_vec.push(cpu);
    }
    cpu_vec
}

/// Get Memory info from system
pub(crate) fn get_memory(system: &mut System) -> Memory {
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

/// Get Load Average Performance from system
fn get_performance() -> LoadPerformance {
    let load = System::load_average();
    LoadPerformance {
        avg_one_min: load.one,
        avg_five_min: load.five,
        avg_fifteen_min: load.fifteen,
    }
}

#[cfg(test)]
mod tests {
    use crate::utils::info::{
        get_cpu, get_disks, get_info, get_memory, get_performance, get_platform,
    };
    use sysinfo::System;

    #[test]
    fn test_get_info() {
        let system_info = get_info();
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
        assert!(system_info.free_memory > 1);
        assert!(system_info.total_memory > 1);
        assert!(system_info.used_memory > 1);
    }

    #[test]
    fn test_get_performance() {
        let system_info = get_performance();
        assert!(system_info.avg_one_min >= 0.0);
        assert!(system_info.avg_five_min >= 0.0);
        assert!(system_info.avg_fifteen_min >= 0.0);
    }

    #[test]
    fn test_get_platform() {
        let plat = get_platform();
        assert_ne!(plat, "Unknown system name")
    }
}
