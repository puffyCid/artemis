use crate::{
    runtime::helper::{bytes_arg, string_arg},
    utils::encoding::{base64_decode_standard, base64_encode_standard},
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string, object::builtins::JsUint8Array};

/// Decode Base64 data
pub(crate) fn js_base64_decode(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let input = string_arg(args, &0)?;

    let decoded_data = match base64_decode_standard(&input) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not get decode input {input}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let bytes = JsUint8Array::from_iter(decoded_data, context)?;

    Ok(bytes.into())
}

/// Encode bytes to Base64 string
pub(crate) fn js_base64_encode(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let input = bytes_arg(args, &0, context)?;
    Ok(js_string!(base64_encode_standard(&input)).into())
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::run::execute_script,
        structs::{artifacts::runtime::script::JSScript, toml::Output},
    };

    fn output_options(name: &str, output: &str, directory: &str, compress: bool) -> Output {
        Output {
            name: name.to_string(),
            directory: directory.to_string(),
            format: String::from("json"),
            compress,
            timeline: false,
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
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2RpbmcvYmFzZTY0LnRzCmZ1bmN0aW9uIGVuY29kZShkYXRhKSB7CiAgY29uc3QgcmVzdWx0ID0ganNfYmFzZTY0X2VuY29kZShkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9lbmNvZGluZy9ieXRlcy50cwpmdW5jdGlvbiBlbmNvZGVCeXRlcyhkYXRhKSB7CiAgY29uc3QgcmVzdWx0ID0ganNfZW5jb2RlX2J5dGVzKGRhdGEpOwogIHJldHVybiByZXN1bHQ7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB0ZXN0ID0gIkRlbm8gaXMgdmVyeSBjb29sISI7CiAgY29uc3QgZGF0YSA9IGVuY29kZShlbmNvZGVCeXRlcyh0ZXN0KSk7CiAgY29uc29sZS5sb2coZGF0YSk7CiAgcmV0dXJuIGRhdGE7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("encode_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_base64_decode() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2RpbmcvYmFzZTY0LnRzCmZ1bmN0aW9uIGRlY29kZShiNjQpIHsKICBjb25zdCBieXRlcyA9IGpzX2Jhc2U2NF9kZWNvZGUoYjY0KTsKICByZXR1cm4gYnl0ZXM7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCB2YWx1ZSA9IGRlY29kZSgiUkdWdWJ5QnBjeUIyWlhKNUlHTnZiMndoIik7CiAgY29uc29sZS5sb2codmFsdWUpOwpyZXR1cm4gQXJyYXkuZnJvbSh2YWx1ZSk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("decode_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
