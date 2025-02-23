use crate::{
    artifacts::os::macos::fsevents::parser::grab_fsventsd_file, runtime::helper::string_arg,
};
use boa_engine::{js_string, Context, JsError, JsResult, JsValue};

/// Expose parsing `FsEvents` to `BoaJS`
pub(crate) fn js_fsevents(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;

    let fsevents = match grab_fsventsd_file(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get fsevents: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&fsevents).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
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
            format: String::from("jsonl"),
            compress,
            url: Some(String::new()),
            api_key: Some(String::new()),
            endpoint_id: String::from("abcd"),
            collection_id: 0,
            output: output.to_string(),
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_js_fsevents() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvZnNldmVudHMudHMKZnVuY3Rpb24gZ2V0RnNldmVudHMocGF0aCkgewogIGNvbnN0IGRhdGEgPSBqc19mc2V2ZW50cyhwYXRoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGF3YWl0IGpzX3JlYWRfZGlyKHBhdGgpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZnNfZGF0YSA9IFtdOwogIGNvbnN0IGZzZXZlbnRzX3BhdGggPSAiL1N5c3RlbS9Wb2x1bWVzL0RhdGEvLmZzZXZlbnRzZCI7CiAgZm9yIChjb25zdCBlbnRyeSBvZiBhd2FpdCByZWFkRGlyKGZzZXZlbnRzX3BhdGgpKSB7CiAgICBpZiAoIWVudHJ5LmlzX2ZpbGUpIHsKICAgICAgY29udGludWU7CiAgICB9CiAgICBjb25zdCBmc2V2ZW50c19maWxlID0gYCR7ZnNldmVudHNfcGF0aH0vJHtlbnRyeS5maWxlbmFtZX1gOwogICAgY29uc3QgaW5mbyA9IGdldEZzZXZlbnRzKGZzZXZlbnRzX2ZpbGUpOwogICAgZm9yIChjb25zdCBmc2V2ZW50X2VudHJ5IG9mIGluZm8pIHsKICAgICAgaWYgKCFmc2V2ZW50X2VudHJ5LnBhdGguaW5jbHVkZXMoIi5ycyIpKSB7CiAgICAgICAgY29udGludWU7CiAgICAgIH0KICAgICAgZnNfZGF0YS5wdXNoKGZzZXZlbnRfZW50cnkpOwogICAgfQogICAgYnJlYWs7CiAgfQogIHJldHVybiBmc19kYXRhOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("fsevent"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
