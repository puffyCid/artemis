use crate::{
    artifacts::os::linux::executable::parser::parse_elf_file, runtime::helper::string_arg,
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};

/// Expose parsing elf file  to `BoaJS`
pub(crate) fn js_get_elf(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;

    let elf_data = match parse_elf_file(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to parse ELF file {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&elf_data).unwrap_or_default();
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
    fn test_get_elf() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbGludXgvZWxmLnRzCmZ1bmN0aW9uIGdldEVsZihwYXRoKSB7CiAgdHJ5IHsKICAgIGNvbnN0IGRhdGEgPSBqc19nZXRfZWxmKHBhdGgpOwogICAgcmV0dXJuIGRhdGE7CiAgfSBjYXRjaCAoZXJyKSB7CiAgICByZXR1cm4gbnVsbDsKICB9Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZGlyZWN0b3J5LnRzCmFzeW5jIGZ1bmN0aW9uIHJlYWREaXIocGF0aCkgewogIGNvbnN0IGRhdGEgPSBhd2FpdCBqc19yZWFkX2RpcihwYXRoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGJpbl9wYXRoID0gIi9iaW4iOwogIGNvbnN0IGVsZnMgPSBbXTsKICBmb3IgKGNvbnN0IGVudHJ5IG9mIGF3YWl0IHJlYWREaXIoYmluX3BhdGgpKSB7CiAgICBpZiAoIWVudHJ5LmlzX2ZpbGUpIHsKICAgICAgY29udGludWU7CiAgICB9CiAgICBjb25zdCBlbGZfcGF0aCA9IGAke2Jpbl9wYXRofS8ke2VudHJ5LmZpbGVuYW1lfWA7CiAgICBjb25zdCBpbmZvID0gZ2V0RWxmKGVsZl9wYXRoKTsKICAgIGlmIChpbmZvID09PSBudWxsKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgY29uc3QgbWV0YSA9IHsKICAgICAgcGF0aDogZWxmX3BhdGgsCiAgICAgIGVsZjogaW5mbywKICAgIH07CiAgICBlbGZzLnB1c2gobWV0YSk7CiAgfQogIHJldHVybiBlbGZzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("elf"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
