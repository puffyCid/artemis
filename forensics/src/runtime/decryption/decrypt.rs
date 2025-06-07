use crate::{runtime::helper::bytes_arg, utils::decryption::decrypt_aes::decrypt_aes_data};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string, object::builtins::JsUint8Array};

/// Decrypt AES256
pub(crate) fn js_decrypt_aes(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let key = bytes_arg(args, &0, context)?;
    let iv = bytes_arg(args, &1, context)?;
    let mut data = bytes_arg(args, &2, context)?;

    let results = match decrypt_aes_data(&key, &iv, &mut data) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not get decrypt data: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let bytes = JsUint8Array::from_iter(results, context)?;

    Ok(bytes.into())
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
    fn test_js_decrypt_aes() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL2RlY3J5cHRpb24vZXJyb3JzLnRzCnZhciBEZWNyeXB0RXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvZGVjcnlwdGlvbi9kZWNyeXB0LnRzCmZ1bmN0aW9uIGRlY3J5cHRfYWVzKGtleSwgaXYsIGRhdGEpIHsKICBjb25zdCBrZXlfbGVuZ3RoID0gMzI7CiAgaWYgKGtleS5sZW5ndGggIT0ga2V5X2xlbmd0aCkgewogICAgcmV0dXJuIG5ldyBEZWNyeXB0RXJyb3IoCiAgICAgIGBBRVNgLAogICAgICBgSW5jb3JyZWN0IGtleSBsZW5ndGgsIHdhbnRlZCAzMiBieXRlcyBnb3Q6ICR7a2V5Lmxlbmd0aH1gCiAgICApOwogIH0KICB0cnkgewogICAgY29uc3QgYnl0ZXMgPSBqc19kZWNyeXB0X2FlcyhrZXksIGl2LCBkYXRhKTsKICAgIHJldHVybiBieXRlczsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgRGVjcnlwdEVycm9yKGBBRVNgLCBgZmFpbGVkIHRvIGRlY3J5cHQ6ICR7ZXJyfWApOwogIH0KfQoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL2VuY29kaW5nL2Vycm9ycy50cwp2YXIgRW5jb2RpbmdFcnJvciA9IGNsYXNzIGV4dGVuZHMgRXJyb3JCYXNlIHsKfTsKCi8vIC4uLy4uL1Byb2plY3RzL2FydGVtaXMtYXBpL3NyYy9lbmNvZGluZy9iYXNlNjQudHMKZnVuY3Rpb24gZGVjb2RlKGI2NCkgewogIHRyeSB7CiAgICBjb25zdCBieXRlcyA9IGpzX2Jhc2U2NF9kZWNvZGUoYjY0KTsKICAgIHJldHVybiBieXRlczsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgRW5jb2RpbmdFcnJvcihgQkFTRTY0YCwgYGZhaWxlZCB0byBkZWNvZGUgJHtiNjR9OiAke2Vycn1gKTsKICB9Cn0KCi8vIC4uLy4uL1Byb2plY3RzL2FydGVtaXMtYXBpL3NyYy9lbmNvZGluZy9zdHJpbmdzLnRzCmZ1bmN0aW9uIGV4dHJhY3RVdGY4U3RyaW5nKGRhdGEpIHsKICBjb25zdCByZXN1bHQgPSBqc19leHRyYWN0X3V0Zjhfc3RyaW5nKGRhdGEpOwogIHJldHVybiByZXN1bHQ7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBkYXRhID0gZGVjb2RlKCJJbEZBOHA5NW9xL0VZOHRIdlI0emZBPT0iKTsKICBjb25zdCB2YWx1ZSA9IGRlY3J5cHRfYWVzKGRlY29kZSgiT05hTEw4MHc0MTZlRFhmMWQrZ2k5V3B0T2drY0o3aERGUG0ydW9qK1JRWT0iKSwgZGVjb2RlKCJNREF3TURBd01EQXdNREF3TURBd01BPT0iKSwgZGF0YSk7CiAgY29uc3QgdGV4dCA9IGV4dHJhY3RVdGY4U3RyaW5nKHZhbHVlKTsKICBpZiAodGV4dCAhPSAiaGVsbG8gQUVTIikgewogICAgdGhyb3cgImJhZCBkZWNyeXB0aW9uISI7CiAgfQogIGNvbnNvbGUubG9nKGBJIGRlY3J5cHRlZCAke3RleHR9YCk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("aes_decrypt"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
