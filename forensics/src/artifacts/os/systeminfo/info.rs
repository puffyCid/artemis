use crate::utils::time::unixepoch_to_iso;
use common::system::{
    Cpus, DiskDrives, LoadPerformance, Memory, NetworkInterface, SystemInfo, SystemInfoMetadata,
};
use std::env;
use sysinfo::{Disks, Networks, Product, System};

/// Get Disk, CPU, Memory, and Performance info from system
pub(crate) fn get_info() -> SystemInfo {
    let mut system = System::new();
    SystemInfo {
        boot_time: unixepoch_to_iso(sysinfo::System::boot_time() as i64),
        hostname: hostname(),
        os_version: sysinfo::System::os_version()
            .unwrap_or_else(|| String::from("Unknown OS version")),
        uptime: sysinfo::System::uptime(),
        kernel_version: sysinfo::System::kernel_version()
            .unwrap_or_else(|| String::from("Unknown kernel version")),
        platform: sysinfo::System::name().unwrap_or_else(|| String::from("Unknown system name")),
        cpu: get_cpu(&mut system),
        disks: get_disks(),
        memory: get_memory(&mut system),
        interfaces: get_network_interfaces(),
        performance: get_performance(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        rust_version: env!("VERGEN_RUSTC_SEMVER").to_string(),
        build_date: env!("VERGEN_BUILD_DATE").to_string(),
        product_name: Product::name().unwrap_or_default(),
        product_family: Product::family().unwrap_or_default(),
        product_serial: Product::serial_number().unwrap_or_default(),
        product_uuid: Product::uuid().unwrap_or_default(),
        product_version: Product::version().unwrap_or_default(),
        vendor: Product::vendor_name().unwrap_or_default(),
    }
}

/// Get some system info
pub(crate) fn get_info_metadata() -> SystemInfoMetadata {
    SystemInfoMetadata {
        hostname: hostname(),
        os_version: sysinfo::System::os_version()
            .unwrap_or_else(|| String::from("Unknown OS Version")),
        platform: sysinfo::System::name().unwrap_or_else(|| String::from("Unknown platform")),
        kernel_version: sysinfo::System::kernel_version()
            .unwrap_or_else(|| String::from("Unknown Kernel Version")),
        performance: get_performance(),
        interfaces: get_network_interfaces(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        rust_version: env!("VERGEN_RUSTC_SEMVER").to_string(),
        build_date: env!("VERGEN_BUILD_DATE").to_string(),
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
        let fs_type = disk.file_system().display().to_string();
        let name = disk.name().display().to_string();
        let disk_data = DiskDrives {
            disk_type: format!("{:?}", disk.kind()),
            file_system: fs_type,
            mount_point: disk.mount_point().display().to_string(),
            total_space: disk.total_space(),
            available_space: disk.available_space(),
            removable: disk.is_removable(),
            name,
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

/// Get network interface cards and IPs
pub(crate) fn get_network_interfaces() -> Vec<NetworkInterface> {
    let net = Networks::new_with_refreshed_list();

    let mut interfaces = Vec::new();
    for (key, network) in &net {
        for ip in network.ip_networks() {
            let interface = NetworkInterface {
                ip: ip.addr.to_string(),
                mac: network.mac_address().to_string(),
                name: key.clone(),
            };
            interfaces.push(interface);
        }
    }

    interfaces
}

pub(crate) fn hostname() -> String {
    sysinfo::System::host_name().unwrap_or_else(|| String::from("Unknown hostname"))
}

#[cfg(test)]
mod tests {
    use crate::artifacts::os::systeminfo::info::{
        get_cpu, get_disks, get_info, get_info_metadata, get_memory, get_network_interfaces,
        get_performance, get_platform, hostname,
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
        assert!(!system_info.is_empty());
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
    fn test_get_info_metadata() {
        let system_info = get_info_metadata();
        assert_eq!(system_info.hostname.is_empty(), false);
        assert_eq!(system_info.platform.is_empty(), false);
        assert_eq!(system_info.kernel_version.is_empty(), false);
        assert_eq!(system_info.os_version.is_empty(), false);
    }

    #[test]
    fn test_get_network_interfaces() {
        let system_info = get_network_interfaces();
        assert!(system_info[0].ip.len() > 1);
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

    #[test]
    fn test_hostname() {
        assert!(!hostname().is_empty())
    }
}
