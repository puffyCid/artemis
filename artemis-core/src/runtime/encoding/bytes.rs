use deno_core::{op, ToJsBuffer};

#[op]
/// Convert string to bytes
fn js_encode_bytes(data: String) -> ToJsBuffer {
    let value = data.as_bytes().to_vec();
    value.into()
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
    fn test_js_encode_bytes() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2RpbmcvYnl0ZXMudHMKZnVuY3Rpb24gZW5jb2RlQnl0ZXMoZGF0YSkgewogIGNvbnN0IHJlc3VsdCA9IGVuY29kaW5nLmJ5dGVzX2VuY29kZShkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgdGVzdCA9ICJEZW5vIGlzIHZlcnkgY29vbCEiOwogIGNvbnN0IGRhdGEgPSBlbmNvZGVCeXRlcyh0ZXN0KTsKICBjb25zb2xlLmxvZyhkYXRhKTsKICByZXR1cm4gQXJyYXkuZnJvbShkYXRhKTsKfQptYWluKCk7Cgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("bytes_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
