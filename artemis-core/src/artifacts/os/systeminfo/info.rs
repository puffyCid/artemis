use serde::Serialize;
use sysinfo::{CpuExt, DiskExt, System, SystemExt};

#[derive(Debug, Serialize)]
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

#[derive(Debug, Serialize)]
pub(crate) struct SystemInfoMetadata {
    pub(crate) hostname: String,
    pub(crate) os_version: String,
    pub(crate) kernel_version: String,
    pub(crate) platform: String,
    pub(crate) performance: LoadPerformance,
}

#[derive(Debug, Serialize)]
pub(crate) struct Cpus {
    pub(crate) frequency: u64,
    pub(crate) cpu_usage: f32,
    pub(crate) name: String,
    pub(crate) vendor_id: String,
    pub(crate) brand: String,
    pub(crate) physical_core_count: usize,
}

#[derive(Debug, Serialize)]
pub(crate) struct Disks {
    pub(crate) disk_type: String,
    pub(crate) file_system: String,
    pub(crate) mount_point: String,
    pub(crate) total_space: u64,
    pub(crate) available_space: u64,
    pub(crate) removable: bool,
}

#[derive(Debug, Serialize)]
pub(crate) struct Memory {
    pub(crate) available_memory: u64,
    pub(crate) free_memory: u64,
    pub(crate) free_swap: u64,
    pub(crate) total_memory: u64,
    pub(crate) total_swap: u64,
    pub(crate) used_memory: u64,
    pub(crate) used_swap: u64,
}

#[derive(Debug, Serialize)]
pub(crate) struct LoadPerformance {
    pub(crate) avg_one_min: f64,
    pub(crate) avg_five_min: f64,
    pub(crate) avg_fifteen_min: f64,
}

impl SystemInfo {
    /// Get Disk, CPU, Memory, and Performance info from system
    pub(crate) fn get_info() -> SystemInfo {
        let mut system = System::new();
        SystemInfo {
            boot_time: system.boot_time(),
            hostname: system
                .host_name()
                .unwrap_or_else(|| String::from("Unknown hostname")),
            os_version: system
                .os_version()
                .unwrap_or_else(|| String::from("Unknown OS Version")),
            uptime: system.uptime(),
            kernel_version: system
                .kernel_version()
                .unwrap_or_else(|| String::from("Unknown Kernel Version")),
            platform: system
                .name()
                .unwrap_or_else(|| String::from("Unknown system name")),
            cpu: SystemInfo::get_cpu(&mut system),
            disks: SystemInfo::get_disks(&mut system),
            memory: SystemInfo::get_memory(&mut system),
            performance: SystemInfo::get_performance(&mut system),
        }
    }

    /// Get some system info
    pub(crate) fn get_info_metadata() -> SystemInfoMetadata {
        let mut system = System::new();
        SystemInfoMetadata {
            hostname: system
                .host_name()
                .unwrap_or_else(|| String::from("Unknown hostname")),
            os_version: system
                .os_version()
                .unwrap_or_else(|| String::from("Unknown OS Version")),
            platform: system
                .name()
                .unwrap_or_else(|| String::from("Unknown platform")),
            kernel_version: system
                .kernel_version()
                .unwrap_or_else(|| String::from("Unknown Kernel Version")),
            performance: SystemInfo::get_performance(&mut system),
        }
    }

    /// Get endpoint platform type
    pub(crate) fn get_platform() -> String {
        let system = System::new();
        system.name().unwrap_or_else(|| String::from("Unknown system name"))
    }
    
    #[cfg(target_os = "windows")]
    /// Get the OS version number
    pub(crate) fn get_os_version() -> String {
        let system = System::new();
        system
            .os_version()
            .unwrap_or_else(|| String::from("Unknown OS Version"))
    }

    #[cfg(target_os = "windows")]
    /// Get the kernel version number
    pub(crate) fn get_win_kernel_version() -> f64 {
        let system = System::new();
        system
            .kernel_version()
            .unwrap_or_else(|| String::from("0.0"))
            .parse::<f64>()
            .unwrap_or(0.0)
    }

    /// Get Disk info from system
    fn get_disks(system: &mut System) -> Vec<Disks> {
        system.refresh_disks_list();
        let disks = system.disks();

        let mut disk_vec: Vec<Disks> = Vec::new();
        for disk in disks {
            let fs_type: Vec<String> = disk.file_system().iter().map(|n| n.to_string()).collect();
            let disk_data = Disks {
                disk_type: format!("{:?}", disk.kind()),
                file_system: fs_type.join(""),
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
        system.refresh_cpu();
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

    /// Get Load Average Performance from system
    fn get_performance(system: &mut System) -> LoadPerformance {
        LoadPerformance {
            avg_one_min: system.load_average().one,
            avg_five_min: system.load_average().five,
            avg_fifteen_min: system.load_average().fifteen,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::SystemInfo;
    use sysinfo::{System, SystemExt};

    #[test]
    fn test_get_info() {
        let system_info = SystemInfo::get_info();
        assert_eq!(system_info.platform.is_empty(), false);
        assert!(system_info.cpu.len() > 1);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_os() {
        let system_info = SystemInfo::get_os_version();
        assert_eq!(system_info.contains("Unknown"), false);
    }

    #[test]
    #[cfg(target_os = "windows")]
    fn test_get_kernel_version() {
        let system_info = SystemInfo::get_win_kernel_version();
        assert!(system_info != 0.0);
    }

    #[test]
    #[cfg(target_os = "macos")]
    fn test_get_macos_disks() {
        let mut system = System::new();

        let system_info = SystemInfo::get_disks(&mut system);
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
        let mut system = System::new();

        let system_info = SystemInfo::get_disks(&mut system);
        assert!(system_info.len() >= 1);
        assert_eq!(system_info[0].disk_type.is_empty(), false);
        assert_eq!(system_info[0].mount_point.contains(":\\"), true);
        assert_eq!(system_info[0].removable, false);
    }

    #[test]
    fn test_get_cpu() {
        let mut system = System::new();

        let system_info = SystemInfo::get_cpu(&mut system);
        assert!(system_info.len() > 1);
    }

    #[test]
    fn test_get_memory() {
        let mut system = System::new();

        let system_info = SystemInfo::get_memory(&mut system);
        assert!(system_info.available_memory > 100);
        assert!(system_info.free_memory > 100);
        assert!(system_info.total_memory > 100);
        assert!(system_info.used_memory > 100);
    }

    #[test]
    fn test_get_info_metadata() {
        let system_info = SystemInfo::get_info_metadata();
        assert_eq!(system_info.hostname.is_empty(), false);
        assert_eq!(system_info.platform.is_empty(), false);
        assert_eq!(system_info.kernel_version.is_empty(), false);
        assert_eq!(system_info.os_version.is_empty(), false);
    }

    #[test]
    fn test_get_performance() {
        let mut system = System::new();

        let system_info = SystemInfo::get_performance(&mut system);
        assert!(system_info.avg_one_min >= 0.0);
        assert!(system_info.avg_five_min >= 0.0);
        assert!(system_info.avg_fifteen_min >= 0.0);
    }

    #[test]
    fn test_get_platform() {
        let plat = SystemInfo::get_platform();
        assert_ne!(plat, "Unknown system name")
    }
}
