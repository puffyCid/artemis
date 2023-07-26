use crate::utils::encoding::{base64_decode_standard, base64_encode_standard};
use deno_core::{error::AnyError, op, JsBuffer, ToJsBuffer};

#[op]
/// Decode Base64 data
fn js_base64_decode(data: String) -> Result<ToJsBuffer, AnyError> {
    let decoded_data = base64_decode_standard(&data)?;
    Ok(decoded_data.into())
}

#[op]
/// Encode bytes to Base64 string
fn js_base64_encode(data: JsBuffer) -> String {
    base64_encode_standard(&data)
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
    fn test_js_base64_encode() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2RpbmcvYmFzZTY0LnRzCmZ1bmN0aW9uIGVuY29kZShkYXRhKSB7CiAgY29uc3QgcmVzdWx0ID0gZW5jb2RpbmcuYnRvYShkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9lbmNvZGluZy9zdHJpbmdzLnRzCmZ1bmN0aW9uIGV4dHJhY3RVdGY4U3RyaW5nKGRhdGEpIHsKICBjb25zdCByZXN1bHQgPSBlbmNvZGluZy5leHRyYWN0X3V0Zjhfc3RyaW5nKGRhdGEpOwogIHJldHVybiByZXN1bHQ7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2VuY29kaW5nL2J5dGVzLnRzCmZ1bmN0aW9uIGVuY29kZUJ5dGVzKGRhdGEpIHsKICBjb25zdCByZXN1bHQgPSBlbmNvZGluZy5ieXRlc19lbmNvZGUoZGF0YSk7CiAgcmV0dXJuIHJlc3VsdDsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHRlc3QgPSAiRGVubyBpcyB2ZXJ5IGNvb2whIjsKICBjb25zdCBkYXRhID0gZW5jb2RlKGVuY29kZUJ5dGVzKHRlc3QpKTsKICBjb25zb2xlLmxvZyhkYXRhKTsKICByZXR1cm4gZGF0YTsKfQptYWluKCk7Cgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("encode_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_base64_decode() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2RpbmcvYmFzZTY0LnRzCmZ1bmN0aW9uIGRlY29kZShiNjQpIHsKICBjb25zdCBieXRlcyA9IGVuY29kaW5nLmF0b2IoYjY0KTsKICByZXR1cm4gYnl0ZXM7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB2YWx1ZSA9IGRlY29kZSgiUkdWdWJ5QnBjeUIyWlhKNUlHTnZiMndoIik7CiAgY29uc29sZS5sb2codmFsdWUpOwpyZXR1cm4gQXJyYXkuZnJvbSh2YWx1ZSk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("decode_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
