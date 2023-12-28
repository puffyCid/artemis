use crate::artifacts::os::systeminfo::info::get_info;
use deno_core::{error::AnyError, op2};

#[op2]
#[string]
/// Expose pulling systeminfo to `Deno`
pub(crate) fn get_systeminfo() -> Result<String, AnyError> {
    let info = get_info();
    let results = serde_json::to_string(&info)?;
    Ok(results)
}

#[op2(fast)]
#[bigint]
/// Return uptime of the system
pub(crate) fn js_uptime() -> u64 {
    sysinfo::System::uptime()
}

#[op2]
#[string]
/// Return hostname of the system
pub(crate) fn js_hostname() -> String {
    sysinfo::System::host_name().unwrap_or_else(|| String::from("Unknown hostname"))
}

#[op2]
#[string]
/// Return OS version of the system
pub(crate) fn js_os_version() -> String {
    sysinfo::System::os_version().unwrap_or_else(|| String::from("Unknown OS version"))
}

#[op2]
#[string]
/// Returns kernel version of the system
pub(crate) fn js_kernel_version() -> String {
    sysinfo::System::kernel_version().unwrap_or_else(|| String::from("Unknown Kernel version"))
}

#[op2]
#[string]
/// Returns the platform of the system
pub(crate) fn js_platform() -> String {
    sysinfo::System::name().unwrap_or_else(|| String::from("Unknown platform"))
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::deno::execute_script, structs::artifacts::runtime::script::JSScript,
        structs::toml::Output,
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_get_systeminfo() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3Mvc3lzdGVtaW5mby50cwpmdW5jdGlvbiBnZXRfc3lzdGVtaW5mb193aW4oKSB7CiAgY29uc3QgZGF0YSA9IERlbm8uY29yZS5vcHMuZ2V0X3N5c3RlbWluZm8oKTsKICBjb25zdCBpbmZvID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gaW5mbzsKfQoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvbW9kLnRzCmZ1bmN0aW9uIGdldFN5c3RlbUluZm9XaW4oKSB7CiAgcmV0dXJuIGdldF9zeXN0ZW1pbmZvX3dpbigpOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgaW5mbyA9IGdldFN5c3RlbUluZm9XaW4oKTsKICByZXR1cm4gaW5mbzsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("systeminfo"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_uptime() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL3N5c3RlbWluZm8udHMKZnVuY3Rpb24gdXB0aW1lKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW0udXB0aW1lKCk7CiAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24gb3NWZXJzaW9uKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW0ub3NWZXJzaW9uKCk7CiAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24ga2VybmVsVmVyc2lvbigpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLmtlcm5lbFZlcnNpb24oKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBwbGF0Zm9ybSgpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLnBsYXRmb3JtKCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9kaXNrcy50cwpmdW5jdGlvbiBkaXNrcygpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLmRpc2tzKCk7CiAgY29uc3QgZGlzayA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGRpc2s7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9jcHUudHMKZnVuY3Rpb24gY3B1cygpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLmNwdSgpOwogIGNvbnN0IGNwdSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGNwdTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL21lbW9yeS50cwpmdW5jdGlvbiBtZW1vcnkoKSB7CiAgY29uc3QgZGF0YSA9IHN5c3RlbS5tZW1vcnkoKTsKICBjb25zdCBtZW0gPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBtZW07Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB0aW1lID0gdXB0aW1lKCk7CiAgY29uc3Qga2VybmVsID0ga2VybmVsVmVyc2lvbigpOwogIGNvbnN0IG9zID0gb3NWZXJzaW9uKCk7CiAgY29uc3QgaW5mbyA9IHBsYXRmb3JtKCk7CiAgY29uc3QgZGlzayA9IGRpc2tzKCk7CiAgY29uc3QgbWVtID0gbWVtb3J5KCk7CiAgY29uc3QgY3B1ID0gY3B1cygpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("systeminfo"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_platform() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL3N5c3RlbWluZm8udHMKZnVuY3Rpb24gdXB0aW1lKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW0udXB0aW1lKCk7CiAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24gb3NWZXJzaW9uKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW0ub3NWZXJzaW9uKCk7CiAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24ga2VybmVsVmVyc2lvbigpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLmtlcm5lbFZlcnNpb24oKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBwbGF0Zm9ybSgpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLnBsYXRmb3JtKCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9kaXNrcy50cwpmdW5jdGlvbiBkaXNrcygpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLmRpc2tzKCk7CiAgY29uc3QgZGlzayA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGRpc2s7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9jcHUudHMKZnVuY3Rpb24gY3B1cygpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLmNwdSgpOwogIGNvbnN0IGNwdSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGNwdTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL21lbW9yeS50cwpmdW5jdGlvbiBtZW1vcnkoKSB7CiAgY29uc3QgZGF0YSA9IHN5c3RlbS5tZW1vcnkoKTsKICBjb25zdCBtZW0gPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBtZW07Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB0aW1lID0gdXB0aW1lKCk7CiAgY29uc3Qga2VybmVsID0ga2VybmVsVmVyc2lvbigpOwogIGNvbnN0IG9zID0gb3NWZXJzaW9uKCk7CiAgY29uc3QgaW5mbyA9IHBsYXRmb3JtKCk7CiAgY29uc3QgZGlzayA9IGRpc2tzKCk7CiAgY29uc3QgbWVtID0gbWVtb3J5KCk7CiAgY29uc3QgY3B1ID0gY3B1cygpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("systeminfo"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
    #[test]
    fn test_js_hostname() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL3N5c3RlbWluZm8udHMKZnVuY3Rpb24gdXB0aW1lKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW1JbmZvLnVwdGltZSgpOwogIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIGhvc3RuYW1lKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW1JbmZvLmhvc3RuYW1lKCk7CiAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24gb3NWZXJzaW9uKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW1JbmZvLm9zVmVyc2lvbigpOwogIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIGtlcm5lbFZlcnNpb24oKSB7CiAgY29uc3QgZGF0YSA9IHN5c3RlbUluZm8ua2VybmVsVmVyc2lvbigpOwogIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIHBsYXRmb3JtKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW1JbmZvLnBsYXRmb3JtKCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9kaXNrcy50cwpmdW5jdGlvbiBkaXNrcygpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtSW5mby5kaXNrcygpOwogIGNvbnN0IGRpc2sgPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBkaXNrOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9zeXN0ZW0vY3B1LnRzCmZ1bmN0aW9uIGNwdXMoKSB7CiAgY29uc3QgZGF0YSA9IHN5c3RlbUluZm8uY3B1KCk7CiAgY29uc3QgY3B1ID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gY3B1Owp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9zeXN0ZW0vbWVtb3J5LnRzCmZ1bmN0aW9uIG1lbW9yeSgpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtSW5mby5tZW1vcnkoKTsKICBjb25zdCBtZW0gPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBtZW07Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB0aW1lID0gdXB0aW1lKCk7CiAgY29uc3Qga2VybmVsID0ga2VybmVsVmVyc2lvbigpOwogIGNvbnN0IG9zID0gb3NWZXJzaW9uKCk7CiAgY29uc3QgaW5mbyA9IHBsYXRmb3JtKCk7CiAgY29uc3QgZGlzayA9IGRpc2tzKCk7CiAgY29uc3QgbWVtID0gbWVtb3J5KCk7CiAgY29uc3QgY3B1ID0gY3B1cygpOwogIGNvbnN0IGhvc3QgPSBob3N0bmFtZSgpOwogIGNvbnNvbGUubG9nKAogICAgYFVwdGltZTogJHt0aW1lfSAtIEtlcm5lbDogJHtrZXJuZWx9IC0gT1M6ICR7b3N9IC0gUGxhdGZvcm06ICR7aW5mb30gLSBIb3N0bmFtZTogJHtob3N0fWAKICApOwogIGNvbnNvbGUubG9nKAogICAgYERpc2tzIFNwYWNlOiAke2Rpc2tbMF0udG90YWxfc3BhY2V9IC0gVG90YWwgTWVtb3J5OiAke21lbS50b3RhbF9tZW1vcnl9IC0gQ1BVIEJyYW5kOiAke2NwdVswXS5icmFuZH1gCiAgKTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("systeminfo"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
    #[test]
    fn test_js_os_version() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL3N5c3RlbWluZm8udHMKZnVuY3Rpb24gdXB0aW1lKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW0udXB0aW1lKCk7CiAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24gb3NWZXJzaW9uKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW0ub3NWZXJzaW9uKCk7CiAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24ga2VybmVsVmVyc2lvbigpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLmtlcm5lbFZlcnNpb24oKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBwbGF0Zm9ybSgpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLnBsYXRmb3JtKCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9kaXNrcy50cwpmdW5jdGlvbiBkaXNrcygpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLmRpc2tzKCk7CiAgY29uc3QgZGlzayA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGRpc2s7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9jcHUudHMKZnVuY3Rpb24gY3B1cygpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLmNwdSgpOwogIGNvbnN0IGNwdSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGNwdTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL21lbW9yeS50cwpmdW5jdGlvbiBtZW1vcnkoKSB7CiAgY29uc3QgZGF0YSA9IHN5c3RlbS5tZW1vcnkoKTsKICBjb25zdCBtZW0gPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBtZW07Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB0aW1lID0gdXB0aW1lKCk7CiAgY29uc3Qga2VybmVsID0ga2VybmVsVmVyc2lvbigpOwogIGNvbnN0IG9zID0gb3NWZXJzaW9uKCk7CiAgY29uc3QgaW5mbyA9IHBsYXRmb3JtKCk7CiAgY29uc3QgZGlzayA9IGRpc2tzKCk7CiAgY29uc3QgbWVtID0gbWVtb3J5KCk7CiAgY29uc3QgY3B1ID0gY3B1cygpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("systeminfo"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
    #[test]
    fn test_js_kernel_version() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL3N5c3RlbWluZm8udHMKZnVuY3Rpb24gdXB0aW1lKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW0udXB0aW1lKCk7CiAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24gb3NWZXJzaW9uKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW0ub3NWZXJzaW9uKCk7CiAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24ga2VybmVsVmVyc2lvbigpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLmtlcm5lbFZlcnNpb24oKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBwbGF0Zm9ybSgpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLnBsYXRmb3JtKCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9kaXNrcy50cwpmdW5jdGlvbiBkaXNrcygpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLmRpc2tzKCk7CiAgY29uc3QgZGlzayA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGRpc2s7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9jcHUudHMKZnVuY3Rpb24gY3B1cygpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLmNwdSgpOwogIGNvbnN0IGNwdSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGNwdTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL21lbW9yeS50cwpmdW5jdGlvbiBtZW1vcnkoKSB7CiAgY29uc3QgZGF0YSA9IHN5c3RlbS5tZW1vcnkoKTsKICBjb25zdCBtZW0gPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBtZW07Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB0aW1lID0gdXB0aW1lKCk7CiAgY29uc3Qga2VybmVsID0ga2VybmVsVmVyc2lvbigpOwogIGNvbnN0IG9zID0gb3NWZXJzaW9uKCk7CiAgY29uc3QgaW5mbyA9IHBsYXRmb3JtKCk7CiAgY29uc3QgZGlzayA9IGRpc2tzKCk7CiAgY29uc3QgbWVtID0gbWVtb3J5KCk7CiAgY29uc3QgY3B1ID0gY3B1cygpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("systeminfo"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
