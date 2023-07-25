use crate::utils::encoding::{base64_decode_standard, base64_encode_standard};
use deno_core::{error::AnyError, op, ByteString};

#[op]
/// Decode Base64 data
fn js_base64_decode(data: String) -> Result<ByteString, AnyError> {
    let decoded_data = base64_decode_standard(&data)?;
    Ok(decoded_data.into())
}

#[op]
/// Encode bytes to Base64 string
fn js_base64_encode(data: ByteString) -> String {
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2RpbmcvYmFzZTY0LnRzCmZ1bmN0aW9uIGVuY29kZShkYXRhKSB7CiAgY29uc3QgcmVzdWx0ID0gZW5jb2RpbmcuYnRvYShkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgdGVzdCA9ICJEZW5vIGlzIHZlcnkgY29vbCEiOwogIGNvbnN0IGRhdGEgPSBlbmNvZGUodGVzdCk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("encode_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_base64_decode() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2RpbmcvYmFzZTY0LnRzCmZ1bmN0aW9uIGRlY29kZShiNjQpIHsKICBjb25zdCBieXRlcyA9IGVuY29kaW5nLmF0b2IoYjY0KTsKICByZXR1cm4gYnl0ZXM7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB2YWx1ZSA9IGRlY29kZSgiUkdWdWJ5QnBjeUIyWlhKNUlHTnZiMndoIik7CiAgY29uc29sZS5sb2codmFsdWUpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("decode_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
