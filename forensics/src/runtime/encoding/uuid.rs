use crate::{
    runtime::helper::bytes_arg,
    utils::uuid::{format_guid_be_bytes, format_guid_le_bytes, generate_uuid},
};
use boa_engine::{Context, JsResult, JsValue, js_string};

/// Attempt to convert bytes to a LE GUID
pub(crate) fn js_format_guid_le_bytes(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let data = bytes_arg(args, 0, context)?;
    Ok(js_string!(format_guid_le_bytes(&data)).into())
}

/// Attempt to convert bytes to a BE GUID
pub(crate) fn js_format_guid_be_bytes(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let data = bytes_arg(args, 0, context)?;
    Ok(js_string!(format_guid_be_bytes(&data)).into())
}

/// Generate a UUID
pub(crate) fn js_generate_uuid(
    _this: &JsValue,
    _args: &[JsValue],
    _context: &mut Context,
) -> JsResult<JsValue> {
    Ok(js_string!(generate_uuid()).into())
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
    fn test_js_uuid() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL2VuY29kaW5nL2Vycm9ycy50cwp2YXIgRW5jb2RpbmdFcnJvciA9IGNsYXNzIGV4dGVuZHMgRXJyb3JCYXNlIHsKfTsKCi8vIC4uLy4uL2FydGVtaXMtYXBpL3NyYy9lbmNvZGluZy9iYXNlNjQudHMKZnVuY3Rpb24gZGVjb2RlKGI2NCkgewogIHRyeSB7CiAgICBjb25zdCBieXRlcyA9IGpzX2Jhc2U2NF9kZWNvZGUoYjY0KTsKICAgIHJldHVybiBieXRlczsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgRW5jb2RpbmdFcnJvcigiQkFTRTY0IiwgYGZhaWxlZCB0byBkZWNvZGUgJHtiNjR9OiAke2Vycn1gKTsKICB9Cn0KCi8vIC4uLy4uL2FydGVtaXMtYXBpL3NyYy9lbmNvZGluZy91dWlkLnRzCmZ1bmN0aW9uIGZvcm1hdEd1aWQoZm9ybWF0LCBkYXRhKSB7CiAgaWYgKGZvcm1hdCA9PT0gMCAvKiBCZSAqLykgewogICAgY29uc3QgcmVzdWx0MiA9IGpzX2Zvcm1hdF9ndWlkX2JlX2J5dGVzKGRhdGEpOwogICAgcmV0dXJuIHJlc3VsdDI7CiAgfQogIGNvbnN0IHJlc3VsdCA9IGpzX2Zvcm1hdF9ndWlkX2xlX2J5dGVzKGRhdGEpOwogIHJldHVybiByZXN1bHQ7Cn0KZnVuY3Rpb24gZ2VuZXJhdGVVdWlkKCkgewogIGNvbnN0IHJlc3VsdCA9IGpzX2dlbmVyYXRlX3V1aWQoKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZGF0YSA9IGRlY29kZSgidFBTQ2xFUGp0a094Y0pwbHZJSXNkdz09Iik7CiAgY29uc3QgZ3VpZCA9IGZvcm1hdEd1aWQoMSAvKiBMZSAqLywgZGF0YSk7CiAgY29uc29sZS5sb2coZ3VpZCk7CiAgY29uc3QgZ3VpZDIgPSBmb3JtYXRHdWlkKDAgLyogQmUgKi8sIGRhdGEpOwogIGNvbnNvbGUubG9nKGd1aWQyKTsKICBjb25zb2xlLmxvZyhnZW5lcmF0ZVV1aWQoKSk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("strings_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
