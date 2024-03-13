use crate::artifacts::os::systeminfo::info::get_disks;
use deno_core::{error::AnyError, op2};

#[op2]
#[string]
/// Return disk info about the system
pub(crate) fn js_disk_info() -> Result<String, AnyError> {
    let disks = get_disks();
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL3N5c3RlbWluZm8udHMKZnVuY3Rpb24gdXB0aW1lKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW0udXB0aW1lKCk7CiAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24gb3NWZXJzaW9uKCkgewogIGNvbnN0IGRhdGEgPSBzeXN0ZW0ub3NWZXJzaW9uKCk7CiAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24ga2VybmVsVmVyc2lvbigpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLmtlcm5lbFZlcnNpb24oKTsKICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBwbGF0Zm9ybSgpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLnBsYXRmb3JtKCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9kaXNrcy50cwpmdW5jdGlvbiBkaXNrcygpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLmRpc2tzKCk7CiAgY29uc3QgZGlzayA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGRpc2s7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3N5c3RlbS9jcHUudHMKZnVuY3Rpb24gY3B1cygpIHsKICBjb25zdCBkYXRhID0gc3lzdGVtLmNwdSgpOwogIGNvbnN0IGNwdSA9IEpTT04ucGFyc2UoZGF0YSk7CiAgcmV0dXJuIGNwdTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvc3lzdGVtL21lbW9yeS50cwpmdW5jdGlvbiBtZW1vcnkoKSB7CiAgY29uc3QgZGF0YSA9IHN5c3RlbS5tZW1vcnkoKTsKICBjb25zdCBtZW0gPSBKU09OLnBhcnNlKGRhdGEpOwogIHJldHVybiBtZW07Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB0aW1lID0gdXB0aW1lKCk7CiAgY29uc3Qga2VybmVsID0ga2VybmVsVmVyc2lvbigpOwogIGNvbnN0IG9zID0gb3NWZXJzaW9uKCk7CiAgY29uc3QgaW5mbyA9IHBsYXRmb3JtKCk7CiAgY29uc3QgZGlzayA9IGRpc2tzKCk7CiAgY29uc3QgbWVtID0gbWVtb3J5KCk7CiAgY29uc3QgY3B1ID0gY3B1cygpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("systeminfo"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
