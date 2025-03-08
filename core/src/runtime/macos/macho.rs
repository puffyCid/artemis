use crate::{artifacts::os::macos::macho::parser::parse_macho, runtime::helper::string_arg};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};

/// Expose parsing macho file  to `BoaJS`
pub(crate) fn js_macho(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let macho = match parse_macho(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get macho: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&macho).unwrap_or_default();
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
            format: String::from("json"),
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
    fn test_js_macho() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbWFjb3MvbWFjaG8udHMKZnVuY3Rpb24gZ2V0TWFjaG8ocGF0aCkgewogIGNvbnN0IGRhdGEgPSBqc19tYWNobyhwYXRoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZmlsZXN5c3RlbS9kaXJlY3RvcnkudHMKYXN5bmMgZnVuY3Rpb24gcmVhZERpcihwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGF3YWl0IGpzX3JlYWRfZGlyKHBhdGgpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmFzeW5jIGZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgYmluX3BhdGggPSAiL2JpbiI7CiAgY29uc3QgbWFjaG9zID0gW107CiAgZm9yIChjb25zdCBlbnRyeSBvZiBhd2FpdCByZWFkRGlyKGJpbl9wYXRoKSkgewogICAgaWYgKCFlbnRyeS5pc19maWxlKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgY29uc3QgbWFjaG9fcGF0aCA9IGAke2Jpbl9wYXRofS8ke2VudHJ5LmZpbGVuYW1lfWA7CiAgICBjb25zdCBpbmZvID0gZ2V0TWFjaG8obWFjaG9fcGF0aCk7CiAgICBpZiAoaW5mbyA9PT0gbnVsbCkgewogICAgICBjb250aW51ZTsKICAgIH0KICAgIGNvbnN0IG1ldGEgPSB7CiAgICAgIHBhdGg6IG1hY2hvX3BhdGgsCiAgICAgIG1hY2hvOiBpbmZvCiAgICB9OwogICAgbWFjaG9zLnB1c2gobWV0YSk7CiAgfQogIHJldHVybiBtYWNob3M7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", true);

        let script = JSScript {
            name: String::from("bin_machos"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
