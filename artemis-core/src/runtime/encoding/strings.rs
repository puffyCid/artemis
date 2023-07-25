use crate::utils::strings::extract_utf8_string;
use deno_core::{op, ByteString};

#[op]
/// Attempt to extract a UTF8 string from raw bytes
fn js_extract_utf8_string(data: ByteString) -> String {
    extract_utf8_string(&data)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::deno::execute_script, structs::artifacts::runtime::script::JSScript,
        utils::artemis_toml::Output,
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2RpbmcvYmFzZTY0LnRzCmZ1bmN0aW9uIGVuY29kZShkYXRhKSB7CiAgY29uc3QgcmVzdWx0ID0gZW5jb2RpbmcuYnRvYShkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CmZ1bmN0aW9uIGRlY29kZShiNjQpIHsKICBjb25zdCBieXRlcyA9IGVuY29kaW5nLmF0b2IoYjY0KTsKICByZXR1cm4gYnl0ZXM7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2VuY29kaW5nL3N0cmluZ3MudHMKZnVuY3Rpb24gZXh0cmFjdFV0ZjhTdHJpbmcoZGF0YSkgewogIGNvbnN0IHJlc3VsdCA9IGVuY29kaW5nLmV4dHJhY3RfdXRmOF9zdHJpbmcoZGF0YSk7CiAgcmV0dXJuIHJlc3VsdDsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHRlc3QgPSAiRGVubyBpcyB2ZXJ5IGNvb2whIjsKICBjb25zdCBkYXRhID0gZW5jb2RlKHRlc3QpOwogIGNvbnN0IHZhbHVlID0gZGVjb2RlKGRhdGEpOwogIGNvbnN0IHJlc3VsdCA9IGV4dHJhY3RVdGY4U3RyaW5nKHZhbHVlKTsKICBjb25zb2xlLmxvZyhyZXN1bHQpOwogIHJldHVybiByZXN1bHQ7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("strings_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
