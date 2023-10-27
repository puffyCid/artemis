use crate::utils::strings::extract_utf8_string;
use deno_core::{op2, JsBuffer};

#[op2]
#[string]
/// Attempt to extract a UTF8 string from raw bytes
pub(crate) fn js_extract_utf8_string(#[buffer] data: JsBuffer) -> String {
    extract_utf8_string(&data)
}

#[op2]
#[string]
/// Attempt to represent bytes as a Hex string
pub(crate) fn js_bytes_to_hex_string(#[buffer] data: JsBuffer) -> String {
    format!("{:02x?}", data)
        .trim_matches('[')
        .trim_matches(']')
        .split(", ")
        .collect()
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
    fn test_js_extract_utf8_string() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2Rpbmcvc3RyaW5ncy50cwpmdW5jdGlvbiBleHRyYWN0VXRmOFN0cmluZyhkYXRhKSB7CiAgY29uc3QgcmVzdWx0ID0gZW5jb2RpbmcuZXh0cmFjdF91dGY4X3N0cmluZyhkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgdmFsdWUgPSBVaW50OEFycmF5LmZyb20oWzc5LCA4MywgODEsIDg1LCA2OSwgODIsIDg5LCA2OCwgNDYsIDY5LCA4OCwgNjksIDBdKTsKICBjb25zdCByZXN1bHQgPSBleHRyYWN0VXRmOFN0cmluZyh2YWx1ZSk7CiAgY29uc29sZS5sb2cocmVzdWx0KTsKICByZXR1cm4gcmVzdWx0Owp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("strings_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_bytes_to_hex_string() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2Rpbmcvc3RyaW5ncy50cwpmdW5jdGlvbiBieXRlc1RvSGV4U3RyaW5nKGRhdGEpIHsKICBjb25zdCByZXN1bHQgPSBlbmNvZGluZy5ieXRlc190b19oZXhfc3RyaW5nKGRhdGEpOwogIHJldHVybiByZXN1bHQ7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB2YWx1ZSA9IFVpbnQ4QXJyYXkuZnJvbShbNzksIDgzLCA4MSwgODUsIDY5LCA4MiwgODksIDY4LCA0NiwgNjksIDg4LCA2OSwgMF0pOwogIGNvbnN0IHJlc3VsdCA9IGJ5dGVzVG9IZXhTdHJpbmcodmFsdWUpOwogIGNvbnNvbGUubG9nKHJlc3VsdCk7CiAgcmV0dXJuIHJlc3VsdDsKfQptYWluKCk7";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("strings_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
