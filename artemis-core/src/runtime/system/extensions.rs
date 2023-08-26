use crate::runtime::system::{
    cpu::js_cpu_info,
    disks::js_disk_info,
    memory::js_memory_info,
    output::output_results,
    processes::get_processes,
    systeminfo::{
        get_systeminfo, js_hostname, js_kernel_version, js_os_version, js_platform, js_uptime,
    },
};
use deno_core::Op;

/// Link Rust functions to `Deno core`
pub(crate) fn system_functions() -> Vec<deno_core::OpDecl> {
    vec![
        get_processes::DECL,
        get_systeminfo::DECL,
        output_results::DECL,
        js_uptime::DECL,
        js_hostname::DECL,
        js_os_version::DECL,
        js_kernel_version::DECL,
        js_platform::DECL,
        js_cpu_info::DECL,
        js_disk_info::DECL,
        js_memory_info::DECL,
    ]
}

#[cfg(test)]
mod tests {
    use super::system_functions;

    #[test]
    fn test_system_functions() {
        let results = system_functions();
        assert!(results.len() > 1)
    }
}
