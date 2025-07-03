use crate::{
    filesystem::ntfs::raw_files::{raw_read_file, read_attribute},
    runtime::helper::string_arg,
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string, object::builtins::JsUint8Array};

/// Expose reading a raw file to `BoaJS`
pub(crate) fn js_read_raw_file(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;

    let data = match raw_read_file(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get read file {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let bytes = JsUint8Array::from_iter(data, context)?;
    Ok(bytes.into())
}

/// Expose reading an alternative data stream (ADS) to `BoaJS`
pub(crate) fn js_read_ads(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;
    let ads_name = string_arg(args, 1)?;

    let data = match read_attribute(&path, &ads_name) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get read ADS data {path}: {err:?}");
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

    #[tokio::test]
    async fn test_read_ads_data_motw() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2RpbmcvYmFzZTY0LnRzCmZ1bmN0aW9uIGRlY29kZShiNjQpIHsKICBjb25zdCBieXRlcyA9IGpzX2Jhc2U2NF9kZWNvZGUoYjY0KTsKICByZXR1cm4gYnl0ZXM7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL3dpbmRvd3MvbnRmcy50cwpmdW5jdGlvbiByZWFkQWRzRGF0YShwYXRoLCBhZHNfbmFtZSkgewogIGNvbnN0IGRhdGEgPSBqc19yZWFkX2FkcygKICAgIHBhdGgsCiAgICBhZHNfbmFtZQogICk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZGlyZWN0b3J5LnRzCmFzeW5jIGZ1bmN0aW9uIHJlYWREaXIocGF0aCkgewogIGNvbnN0IGRhdGEgPSBhd2FpdCBqc19yZWFkX2RpcihwYXRoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBzdGF0KHBhdGgpIHsKICBjb25zdCBkYXRhID0ganNfc3RhdChwYXRoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW52aXJvbm1lbnQvZW52LnRzCmZ1bmN0aW9uIGdldEVudlZhbHVlKGtleSkgewogIGNvbnN0IGRhdGEgPSBqc19lbnZfdmFsdWUoa2V5KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2Rpbmcvc3RyaW5ncy50cwpmdW5jdGlvbiBleHRyYWN0VXRmOFN0cmluZyhkYXRhKSB7CiAgY29uc3QgcmVzdWx0ID0ganNfZXh0cmFjdF91dGY4X3N0cmluZyhkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZHJpdmUgPSBnZXRFbnZWYWx1ZSgiU3lzdGVtRHJpdmUiKTsKICBpZiAoZHJpdmUgPT09ICIiKSB7CiAgICByZXR1cm4gW107CiAgfQogIGNvbnN0IHdlYl9maWxlcyA9IFtdOwogIGNvbnN0IHVzZXJzID0gYCR7ZHJpdmV9XFxVc2Vyc2A7CiAgZm9yIChjb25zdCBlbnRyeSBvZiBhd2FpdCByZWFkRGlyKHVzZXJzKSkgewogICAgdHJ5IHsKICAgICAgY29uc3QgcGF0aCA9IGAke3VzZXJzfVxcJHtlbnRyeS5maWxlbmFtZX1cXERvd25sb2Fkc2A7CiAgICAgIGZvciAoY29uc3QgZmlsZV9lbnRyeSBvZiBhd2FpdCByZWFkRGlyKHBhdGgpKSB7CiAgICAgICAgdHJ5IHsKICAgICAgICAgIGlmICghZmlsZV9lbnRyeS5pc19maWxlKSB7CiAgICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgICAgfQogICAgICAgICAgY29uc3QgZnVsbF9wYXRoID0gYCR7cGF0aH1cXCR7ZmlsZV9lbnRyeS5maWxlbmFtZX1gOwogICAgICAgICAgY29uc3QgYWRzID0gIlpvbmUuSWRlbnRpZmllciI7CiAgICAgICAgICBjb25zdCBkYXRhID0gcmVhZEFkc0RhdGEoZnVsbF9wYXRoLCBhZHMpOwogICAgICAgICAgaWYgKGRhdGEubGVuZ3RoID09PSAwKSB7CiAgICAgICAgICAgIGNvbnRpbnVlOwogICAgICAgICAgfQogICAgICAgICAgY29uc3QgaW5mbyA9IHN0YXQoZnVsbF9wYXRoKTsKICAgICAgICAgIGNvbnN0IG1hcmtfaW5mbyA9IGV4dHJhY3RVdGY4U3RyaW5nKGRhdGEpOwogICAgICAgICAgY29uc3Qgd2ViX2ZpbGUgPSB7CiAgICAgICAgICAgIG1hcms6IG1hcmtfaW5mbywKICAgICAgICAgICAgcGF0aDogZnVsbF9wYXRoLAogICAgICAgICAgICBjcmVhdGVkOiBpbmZvLmNyZWF0ZWQsCiAgICAgICAgICAgIG1vZGlmaWVkOiBpbmZvLm1vZGlmaWVkLAogICAgICAgICAgICBhY2Nlc3NlZDogaW5mby5hY2Nlc3NlZCwKICAgICAgICAgICAgc2l6ZTogaW5mby5zaXplCiAgICAgICAgICB9OwogICAgICAgICAgd2ViX2ZpbGVzLnB1c2god2ViX2ZpbGUpOwogICAgICAgIH0gY2F0Y2ggKF9lcnJvcikgewogICAgICAgICAgY29udGludWU7CiAgICAgICAgfQogICAgICB9CiAgICB9IGNhdGNoIChfZXJyb3IpIHsKICAgICAgY29udGludWU7CiAgICB9CiAgfQogIHJldHVybiB3ZWJfZmlsZXM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("read_ads_motw"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).await.unwrap();
    }

    #[tokio::test]
    async fn test_read_raw_file_swapfile() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9udGZzLnRzCmZ1bmN0aW9uIHJlYWRSYXdGaWxlKHBhdGgpIHsKICBjb25zdCBkYXRhID0ganNfcmVhZF9yYXdfZmlsZShwYXRoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW52aXJvbm1lbnQvZW52LnRzCmZ1bmN0aW9uIGdldEVudlZhbHVlKGtleSkgewogIGNvbnN0IGRhdGEgPSBqc19lbnZfdmFsdWUoa2V5KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9maWxlcy50cwpmdW5jdGlvbiBzdGF0KHBhdGgpIHsKICBjb25zdCBkYXRhID0ganNfc3RhdChwYXRoKTsKICByZXR1cm4gdmFsdWU7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBkcml2ZSA9IGdldEVudlZhbHVlKCJTeXN0ZW1Ecml2ZSIpOwogIGlmIChkcml2ZSA9PT0gIiIpIHsKICAgIHJldHVybiAwOwogIH0KICB0cnkgewogICAgY29uc3Qgc3dhcCA9IGAke2RyaXZlfVxcc3dhcGZpbGUuc3lzYDsKICAgIGNvbnN0IGluZm8gPSBzdGF0KHN3YXApOwogICAgaWYgKCFpbmZvLmlzX2ZpbGUpIHsKICAgICAgcmV0dXJuIDA7CiAgICB9CiAgICBjb25zdCBtYXhfc2l6ZSA9IDIxNDc0ODM2NDg7CiAgICBpZiAoaW5mby5zaXplID4gbWF4X3NpemUpIHsKICAgICAgcmV0dXJuIDA7CiAgICB9CiAgICBjb25zdCBkYXRhID0gcmVhZFJhd0ZpbGUoc3dhcCk7CiAgICByZXR1cm4gZGF0YS5sZW5ndGg7CiAgfSBjYXRjaCAoX2Vycm9yKSB7CiAgICByZXR1cm4gMDsKICB9Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("swapfile"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).await.unwrap();
    }
}
