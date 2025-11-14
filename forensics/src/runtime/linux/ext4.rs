use std::{fs::File, io::BufReader};

use crate::{
    filesystem::ext4::raw_files::{raw_read_dir, raw_read_file, raw_read_inode},
    runtime::helper::{number_arg, string_arg},
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string, object::builtins::JsUint8Array};
use ext4_fs::extfs::Ext4Reader;

/// Expose reading the raw ext4 filesystem to `BoaJS`
pub(crate) fn js_read_raw_file_ext4(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;
    let device = string_arg(args, 1)?;

    let data = match raw_read_file(&path, Some(&device)) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to raw read file {path} for device {device}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let bytes = JsUint8Array::from_iter(data, context)?;
    Ok(bytes.into())
}

/// Expose reading the raw directories ext4 filesystem to `BoaJS`
pub(crate) fn js_read_raw_dir_ext4(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;
    let start = string_arg(args, 1)?;
    let device = string_arg(args, 2)?;

    let data = match raw_read_dir(&path, &start, Some(&device)) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!(
                "Failed to raw read directory {path} at {start} for device {device}: {err:?}"
            );
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&data).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;
    Ok(value)
}

/// Expose reading the raw ext4 filesystem inode to `BoaJS`
pub(crate) fn js_read_raw_inode_ext4(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let inode = number_arg(args, 0)? as u32;
    let device = string_arg(args, 1)?;
    let reader = match File::open(&device) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not open ext4 device ({device}): {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let buf = BufReader::new(reader);
    let mut ext_reader = match Ext4Reader::new(buf, 4096, 0) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not create ext4 reader for device ({device}): {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let data = match raw_read_inode(inode, &mut ext_reader) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to raw read inode {inode} for device {device}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let bytes = JsUint8Array::from_iter(data, context)?;
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
    fn test_js_read_raw_file_ext4() {
        let test = "KCgpID0+IHsKICAvLyAuLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvdXRpbHMvZXJyb3IudHMKICB2YXIgRXJyb3JCYXNlID0gY2xhc3MgZXh0ZW5kcyBFcnJvciB7CiAgICBuYW1lOwogICAgbWVzc2FnZTsKICAgIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgICAgc3VwZXIoKTsKICAgICAgdGhpcy5uYW1lID0gbmFtZTsKICAgICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICAgIH0KICB9OwoKICAvLyAuLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvbGludXgvZXJyb3JzLnRzCiAgdmFyIExpbnV4RXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7CiAgfTsKCiAgLy8gLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3N5c3RlbS9zeXN0ZW1pbmZvLnRzCiAgZnVuY3Rpb24gZ2V0U3lzdGVtaW5mbygpIHsKICAgIGNvbnN0IGRhdGEgPSBqc19nZXRfc3lzdGVtaW5mbygpOwogICAgcmV0dXJuIGRhdGE7CiAgfQoKICAvLyAuLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvbGludXgvZXh0NC50cwogIGZ1bmN0aW9uIHJlYWRSYXdGaWxlRXh0NChwYXRoLCBkZXZpY2UpIHsKICAgIHRyeSB7CiAgICAgIGNvbnN0IGRhdGEgPSBqc19yZWFkX3Jhd19maWxlX2V4dDQocGF0aCwgZGV2aWNlKTsKICAgICAgcmV0dXJuIGRhdGE7CiAgICB9IGNhdGNoIChlcnIpIHsKICAgICAgcmV0dXJuIG5ldyBMaW51eEVycm9yKCJFWFQ0IiwgYGZhaWxlZCB0byByZWFkIGZpbGUgJHtwYXRofTogJHtlcnJ9YCk7CiAgICB9CiAgfQogIGZ1bmN0aW9uIHJlYWRSYXdEaXJFeHQ0KHBhdGgsIHN0YXJ0LCBkZXZpY2UpIHsKICAgIHRyeSB7CiAgICAgIGNvbnN0IGRhdGEgPSBqc19yZWFkX3Jhd19kaXJfZXh0NChwYXRoLCBzdGFydCwgZGV2aWNlKTsKICAgICAgcmV0dXJuIGRhdGE7CiAgICB9IGNhdGNoIChlcnIpIHsKICAgICAgcmV0dXJuIG5ldyBMaW51eEVycm9yKCJFWFQ0IiwgYGZhaWxlZCB0byByZWFkIHBhdGggJHtwYXRofTogJHtlcnJ9YCk7CiAgICB9CiAgfQogIGZ1bmN0aW9uIHJlYWRSYXdJbm9kZUV4dDQoaW5vZGUsIGRldmljZSkgewogICAgaWYgKGlub2RlIDw9IDApIHsKICAgICAgcmV0dXJuIG5ldyBMaW51eEVycm9yKGBFWFQ0YCwgYFlvdSBwcm92aWRlZCBhIGJpemFycmUgaW5vZGUgbnVtYmVyPyBJdCBtdXN0IGJlIGdyZWF0ZXIgdGhhbiAwYCk7CiAgICB9CiAgICB0cnkgewogICAgICBjb25zdCBkYXRhID0ganNfcmVhZF9yYXdfaW5vZGVfZXh0NChpbm9kZSwgZGV2aWNlKTsKICAgICAgcmV0dXJuIGRhdGE7CiAgICB9IGNhdGNoIChlcnIpIHsKICAgICAgcmV0dXJuIG5ldyBMaW51eEVycm9yKCJFWFQ0IiwgYGZhaWxlZCB0byByZWFkIGlub2RlICR7aW5vZGV9OiAke2Vycn1gKTsKICAgIH0KICB9CgogIC8vIG1haW4udHMKICBmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgaW5mbyA9IGdldFN5c3RlbWluZm8oKTsKICAgIGZvciAoY29uc3QgZW50cnkgb2YgaW5mby5kaXNrcykgewogICAgICBpZiAoZW50cnkuZmlsZV9zeXN0ZW0udG9Mb3dlckNhc2UoKSAhPT0gImV4dDQiKSB7CiAgICAgICAgY29udGludWU7CiAgICAgIH0KICAgICAgY29uc3Qgc3RhcnQgPSAiL2Jvb3QiOwogICAgICBjb25zdCBwYXRoID0gIi4qLmltZyI7CiAgICAgIGNvbnN0IHZhbHVlcyA9IHJlYWRSYXdEaXJFeHQ0KHBhdGgsIHN0YXJ0LCBlbnRyeS5uYW1lKTsKICAgICAgaWYgKHZhbHVlcyBpbnN0YW5jZW9mIExpbnV4RXJyb3IpIHsKICAgICAgICBjb250aW51ZTsKICAgICAgfQogICAgICBjb25zb2xlLmxvZyhgR290ICR7dmFsdWVzLmxlbmd0aH0gbWF0Y2hlcyFgKTsKICAgICAgZm9yIChjb25zdCBmaWxlIG9mIHZhbHVlcykgewogICAgICAgIGlmIChmaWxlLmZpbGVfdHlwZSAhPT0gIkZpbGUiIC8qIEZpbGUgKi8pIHsKICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgIH0KICAgICAgICBjb25zb2xlLmxvZyhmaWxlLmZ1bGxfcGF0aCk7CiAgICAgICAgY29uc3QgYnl0ZXMgPSByZWFkUmF3SW5vZGVFeHQ0KGZpbGUuaW5vZGUsIGVudHJ5Lm5hbWUpOwogICAgICAgIGlmIChieXRlcyBpbnN0YW5jZW9mIExpbnV4RXJyb3IpIHsKICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgIH0KICAgICAgICBjb25zdCBieXRlczIgPSByZWFkUmF3RmlsZUV4dDQoZmlsZS5mdWxsX3BhdGgsIGVudHJ5Lm5hbWUpOwogICAgICAgIGlmIChieXRlczIgaW5zdGFuY2VvZiBMaW51eEVycm9yKSB7CiAgICAgICAgICBjb250aW51ZTsKICAgICAgICB9CiAgICAgICAgaWYgKGJ5dGVzLmxlbmd0aCAhPT0gYnl0ZXMyLmxlbmd0aCkgewogICAgICAgICAgY29uc29sZS5lcnJvcigicmVhZCBhIGZpbGUgdHdpY2UgYnV0IGdvdCBkaWZmZXJlbnQgYnl0ZXM/IElzIHRoZSBPUyBjb25zdGFudGx5IG1vZGlmeWluZyBpdD8iKTsKICAgICAgICB9CiAgICAgIH0KICAgIH0KICB9CiAgbWFpbigpOwp9KSgpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("read_ads_motw"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_read_raw_inode_ext4() {
        let test = "KCgpID0+IHsKICAvLyAuLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvdXRpbHMvZXJyb3IudHMKICB2YXIgRXJyb3JCYXNlID0gY2xhc3MgZXh0ZW5kcyBFcnJvciB7CiAgICBuYW1lOwogICAgbWVzc2FnZTsKICAgIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgICAgc3VwZXIoKTsKICAgICAgdGhpcy5uYW1lID0gbmFtZTsKICAgICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICAgIH0KICB9OwoKICAvLyAuLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvbGludXgvZXJyb3JzLnRzCiAgdmFyIExpbnV4RXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7CiAgfTsKCiAgLy8gLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3N5c3RlbS9zeXN0ZW1pbmZvLnRzCiAgZnVuY3Rpb24gZ2V0U3lzdGVtaW5mbygpIHsKICAgIGNvbnN0IGRhdGEgPSBqc19nZXRfc3lzdGVtaW5mbygpOwogICAgcmV0dXJuIGRhdGE7CiAgfQoKICAvLyAuLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvbGludXgvZXh0NC50cwogIGZ1bmN0aW9uIHJlYWRSYXdEaXJFeHQ0KHBhdGgsIHN0YXJ0LCBkZXZpY2UpIHsKICAgIHRyeSB7CiAgICAgIGNvbnN0IGRhdGEgPSBqc19yZWFkX3Jhd19kaXJfZXh0NChwYXRoLCBzdGFydCwgZGV2aWNlKTsKICAgICAgcmV0dXJuIGRhdGE7CiAgICB9IGNhdGNoIChlcnIpIHsKICAgICAgcmV0dXJuIG5ldyBMaW51eEVycm9yKCJFWFQ0IiwgYGZhaWxlZCB0byByZWFkIHBhdGggJHtwYXRofTogJHtlcnJ9YCk7CiAgICB9CiAgfQogIGZ1bmN0aW9uIHJlYWRSYXdJbm9kZUV4dDQoaW5vZGUsIGRldmljZSkgewogICAgaWYgKGlub2RlIDw9IDApIHsKICAgICAgcmV0dXJuIG5ldyBMaW51eEVycm9yKGBFWFQ0YCwgYFlvdSBwcm92aWRlZCBhIGJpemFycmUgaW5vZGUgbnVtYmVyPyBJdCBtdXN0IGJlIGdyZWF0ZXIgdGhhbiAwYCk7CiAgICB9CiAgICB0cnkgewogICAgICBjb25zdCBkYXRhID0ganNfcmVhZF9yYXdfaW5vZGVfZXh0NChpbm9kZSwgZGV2aWNlKTsKICAgICAgcmV0dXJuIGRhdGE7CiAgICB9IGNhdGNoIChlcnIpIHsKICAgICAgcmV0dXJuIG5ldyBMaW51eEVycm9yKCJFWFQ0IiwgYGZhaWxlZCB0byByZWFkIGlub2RlICR7aW5vZGV9OiAke2Vycn1gKTsKICAgIH0KICB9CgogIC8vIG1haW4udHMKICBmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgaW5mbyA9IGdldFN5c3RlbWluZm8oKTsKICAgIGZvciAoY29uc3QgZW50cnkgb2YgaW5mby5kaXNrcykgewogICAgICBpZiAoZW50cnkuZmlsZV9zeXN0ZW0udG9Mb3dlckNhc2UoKSAhPT0gImV4dDQiKSB7CiAgICAgICAgY29udGludWU7CiAgICAgIH0KICAgICAgY29uc3Qgc3RhcnQgPSAiL2Jvb3QiOwogICAgICBjb25zdCBwYXRoID0gIi4qLmltZyI7CiAgICAgIGNvbnN0IHZhbHVlcyA9IHJlYWRSYXdEaXJFeHQ0KHBhdGgsIHN0YXJ0LCBlbnRyeS5uYW1lKTsKICAgICAgaWYgKHZhbHVlcyBpbnN0YW5jZW9mIExpbnV4RXJyb3IpIHsKICAgICAgICBjb250aW51ZTsKICAgICAgfQogICAgICBjb25zb2xlLmxvZyhgR290ICR7dmFsdWVzLmxlbmd0aH0gbWF0Y2hlcyFgKTsKICAgICAgZm9yIChjb25zdCBmaWxlIG9mIHZhbHVlcykgewogICAgICAgIGlmIChmaWxlLmZpbGVfdHlwZSAhPT0gIkZpbGUiIC8qIEZpbGUgKi8pIHsKICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgIH0KICAgICAgICBjb25zb2xlLmxvZyhmaWxlLmZ1bGxfcGF0aCk7CiAgICAgICAgY29uc3QgYnl0ZXMgPSByZWFkUmF3SW5vZGVFeHQ0KGZpbGUuaW5vZGUsIGVudHJ5Lm5hbWUpOwogICAgICAgIGlmIChieXRlcyBpbnN0YW5jZW9mIExpbnV4RXJyb3IpIHsKICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgIH0KICAgICAgfQogICAgfQogIH0KICBtYWluKCk7Cn0pKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("swapfile"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_read_raw_dir_ext4() {
        let test = "KCgpID0+IHsKICAvLyAuLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvdXRpbHMvZXJyb3IudHMKICB2YXIgRXJyb3JCYXNlID0gY2xhc3MgZXh0ZW5kcyBFcnJvciB7CiAgICBuYW1lOwogICAgbWVzc2FnZTsKICAgIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgICAgc3VwZXIoKTsKICAgICAgdGhpcy5uYW1lID0gbmFtZTsKICAgICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICAgIH0KICB9OwoKICAvLyAuLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvbGludXgvZXJyb3JzLnRzCiAgdmFyIExpbnV4RXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7CiAgfTsKCiAgLy8gLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3N5c3RlbS9zeXN0ZW1pbmZvLnRzCiAgZnVuY3Rpb24gZ2V0U3lzdGVtaW5mbygpIHsKICAgIGNvbnN0IGRhdGEgPSBqc19nZXRfc3lzdGVtaW5mbygpOwogICAgcmV0dXJuIGRhdGE7CiAgfQoKICAvLyAuLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvbGludXgvZXh0NC50cwogIGZ1bmN0aW9uIHJlYWRSYXdEaXJFeHQ0KHBhdGgsIHN0YXJ0LCBkZXZpY2UpIHsKICAgIHRyeSB7CiAgICAgIGNvbnN0IGRhdGEgPSBqc19yZWFkX3Jhd19kaXJfZXh0NChwYXRoLCBzdGFydCwgZGV2aWNlKTsKICAgICAgcmV0dXJuIGRhdGE7CiAgICB9IGNhdGNoIChlcnIpIHsKICAgICAgcmV0dXJuIG5ldyBMaW51eEVycm9yKCJFWFQ0IiwgYGZhaWxlZCB0byByZWFkIHBhdGggJHtwYXRofTogJHtlcnJ9YCk7CiAgICB9CiAgfQoKICAvLyBtYWluLnRzCiAgZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IGluZm8gPSBnZXRTeXN0ZW1pbmZvKCk7CiAgICBmb3IgKGNvbnN0IGVudHJ5IG9mIGluZm8uZGlza3MpIHsKICAgICAgaWYgKGVudHJ5LmZpbGVfc3lzdGVtLnRvTG93ZXJDYXNlKCkgIT09ICJleHQ0IikgewogICAgICAgIGNvbnRpbnVlOwogICAgICB9CiAgICAgIGNvbnN0IHN0YXJ0ID0gIi9ib290IjsKICAgICAgY29uc3QgcGF0aCA9ICIuKi5pbWciOwogICAgICBjb25zdCB2YWx1ZXMgPSByZWFkUmF3RGlyRXh0NChwYXRoLCBzdGFydCwgZW50cnkubmFtZSk7CiAgICAgIGlmICh2YWx1ZXMgaW5zdGFuY2VvZiBMaW51eEVycm9yKSB7CiAgICAgICAgY29udGludWU7CiAgICAgIH0KICAgICAgY29uc29sZS5sb2coYEdvdCAke3ZhbHVlcy5sZW5ndGh9IG1hdGNoZXMhYCk7CiAgICB9CiAgfQogIG1haW4oKTsKfSkoKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("read_ads_motw"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
