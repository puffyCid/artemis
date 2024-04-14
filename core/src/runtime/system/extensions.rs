use crate::runtime::system::{
    command::js_command,
    cpu::js_cpu_info,
    disks::js_disk_info,
    logging::js_log,
    memory::js_memory_info,
    output::{output_results, raw_dump},
    processes::get_processes,
    systeminfo::{
        get_systeminfo, js_hostname, js_kernel_version, js_os_version, js_platform, js_uptime,
    },
};

/// Link Rust functions to `Deno core`
pub(crate) fn system_functions() -> Vec<deno_core::OpDecl> {
    vec![
        get_processes(),
        get_systeminfo(),
        output_results(),
        raw_dump(),
        js_uptime(),
        js_hostname(),
        js_os_version(),
        js_kernel_version(),
        js_platform(),
        js_cpu_info(),
        js_disk_info(),
        js_memory_info(),
        js_command(),
        js_log(),
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
