use crate::artifacts::os::systeminfo::info::SystemInfo;
use deno_core::{error::AnyError, op};
use sysinfo::{System, SystemExt};

#[op]
/// Return disk info about the system
fn js_disk_info() -> Result<String, AnyError> {
    let mut info = System::new();
    let disks = SystemInfo::get_disks(&mut info);
    let results = serde_json::to_string(&disks)?;
    Ok(results)
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
    fn test_js_disk_info() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL3N5c3RlbWluZm8udHMKZnVuY3Rpb24gdXB0aW1lKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW1JbmZvLnVwdGltZSgpOwogIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIG9zVmVyc2lvbigpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtSW5mby5vc1ZlcnNpb24oKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBrZXJuZWxWZXJzaW9uKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW1JbmZvLmtlcm5lbFZlcnNpb24oKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBwbGF0Zm9ybSgpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtSW5mby5wbGF0Zm9ybSgpOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9zeXN0ZW0vZGlza3MudHMKZnVuY3Rpb24gZGlza3MoKSB7CiAgY29uc3QgZGF0YSA9IHN5c3RlbUluZm8uZGlza3MoKTsKICBjb25zdCBkaXNrID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gZGlzazsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL2NwdS50cwpmdW5jdGlvbiBjcHVzKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW1JbmZvLmNwdSgpOwogIGNvbnN0IGNwdSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGNwdTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL21lbW9yeS50cwpmdW5jdGlvbiBtZW1vcnkoKSB7CiAgY29uc3QgZGF0YSA9IHN5c3RlbUluZm8ubWVtb3J5KCk7CiAgY29uc3QgbWVtID0gSlNPTi5wYXJzZShkYXRhKTsKICByZXR1cm4gbWVtOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgdGltZSA9IHVwdGltZSgpOwogIGNvbnN0IGtlcm5lbCA9IGtlcm5lbFZlcnNpb24oKTsKICBjb25zdCBvcyA9IG9zVmVyc2lvbigpOwogIGNvbnN0IGluZm8gPSBwbGF0Zm9ybSgpOwogIGNvbnN0IGRpc2sgPSBkaXNrcygpOwogIGNvbnN0IG1lbSA9IG1lbW9yeSgpOwogIGNvbnN0IGNwdSA9IGNwdXMoKTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("systeminfo"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
