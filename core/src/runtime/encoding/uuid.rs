use crate::utils::uuid::{format_guid_be_bytes, format_guid_le_bytes, generate_uuid};
use deno_core::{op2, JsBuffer};

#[op2]
#[string]
/// Attempt to convert bytes to a LE GUID
pub(crate) fn js_format_guid_le_bytes(#[buffer] data: JsBuffer) -> String {
    format_guid_le_bytes(&data)
}

#[op2]
#[string]
/// Attempt to convert bytes to a BE GUID
pub(crate) fn js_format_guid_be_bytes(#[buffer] data: JsBuffer) -> String {
    format_guid_be_bytes(&data)
}

#[op2]
#[string]
/// Generate a UUID
pub(crate) fn js_generate_uuid() -> String {
    generate_uuid()
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
    fn test_js_uuid() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2VuY29kaW5nL2Vycm9ycy50cwp2YXIgRW5jb2RpbmdFcnJvciA9IGNsYXNzIGV4dGVuZHMgRXJyb3JCYXNlIHsKfTsKCi8vIC4uLy4uL2FydGVtaXMtYXBpL3NyYy9lbmNvZGluZy9iYXNlNjQudHMKZnVuY3Rpb24gZGVjb2RlKGI2NCkgewogIHRyeSB7CiAgICBjb25zdCBieXRlcyA9IGVuY29kaW5nLmF0b2IoYjY0KTsKICAgIHJldHVybiBieXRlczsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgRW5jb2RpbmdFcnJvcigiQkFTRTY0IiwgYGZhaWxlZCB0byBkZWNvZGUgJHtiNjR9OiAke2Vycn1gKTsKICB9Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL3NyYy9lbmNvZGluZy91dWlkLnRzCmZ1bmN0aW9uIGZvcm1hdEd1aWQoZm9ybWF0LCBkYXRhKSB7CiAgaWYgKGZvcm1hdCA9PT0gMCAvKiBCZSAqLykgewogICAgY29uc3QgcmVzdWx0MiA9IGVuY29kaW5nLmJ5dGVzX3RvX2JlX2d1aWQoZGF0YSk7CiAgICByZXR1cm4gcmVzdWx0MjsKICB9CiAgY29uc3QgcmVzdWx0ID0gZW5jb2RpbmcuYnl0ZXNfdG9fbGVfZ3VpZChkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CmZ1bmN0aW9uIGdlbmVyYXRlVXVpZCgpIHsKICBjb25zdCByZXN1bHQgPSBlbmNvZGluZy5nZW5lcmF0ZV91dWlkKCk7CiAgcmV0dXJuIHJlc3VsdDsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGRhdGEgPSBkZWNvZGUoInRQU0NsRVBqdGtPeGNKcGx2SUlzZHc9PSIpOwogIGNvbnN0IGd1aWQgPSBmb3JtYXRHdWlkKDEgLyogTGUgKi8sIGRhdGEpOwogIGNvbnNvbGUubG9nKGd1aWQpOwogIGNvbnN0IGd1aWQyID0gZm9ybWF0R3VpZCgwIC8qIEJlICovLCBkYXRhKTsKICBjb25zb2xlLmxvZyhndWlkMik7CiAgY29uc29sZS5sb2coZ2VuZXJhdGVVdWlkKCkpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("strings_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
