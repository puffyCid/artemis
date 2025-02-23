use crate::{
    artifacts::os::linux::journals::parser::grab_journal_file, runtime::helper::string_arg,
};
use boa_engine::{js_string, Context, JsError, JsResult, JsValue};

/// Expose parsing journal file  to `BoaJS`
pub(crate) fn js_get_journal(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;

    let journal_data = match grab_journal_file(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to parse journal file {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&journal_data).unwrap_or_default();
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
            filter_name: None,
            filter_script: None,
            logging: None,
        }
    }

    #[test]
    fn test_js_get_journal() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbGludXgvam91cm5hbC50cwpmdW5jdGlvbiBnZXRKb3VybmFsKHBhdGgpIHsKICBjb25zdCBkYXRhID0ganNfZ2V0X2pvdXJuYWwocGF0aCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL2ZpbGVzeXN0ZW0vZGlyZWN0b3J5LnRzCmFzeW5jIGZ1bmN0aW9uIHJlYWREaXIocGF0aCkgewogIGNvbnN0IGRhdGEgPSBhd2FpdCBqc19yZWFkX2RpcihwYXRoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGpvdXJuYWxzID0gIi92YXIvbG9nL2pvdXJuYWwiOwogIGZvciAoY29uc3QgZW50cnkgb2YgYXdhaXQgcmVhZERpcihqb3VybmFscykpIHsKICAgIGlmICghZW50cnkuaXNfZGlyZWN0b3J5KSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgY29uc3QgZnVsbF9wYXRoID0gYCR7am91cm5hbHN9LyR7ZW50cnkuZmlsZW5hbWV9YDsKICAgIGZvciAoY29uc3QgZmlsZXMgb2YgYXdhaXQgcmVhZERpcihmdWxsX3BhdGgpKSB7CiAgICAgIGlmICghZmlsZXMuZmlsZW5hbWUuZW5kc1dpdGgoImpvdXJuYWwiKSkgewogICAgICAgIGNvbnRpbnVlOwogICAgICB9CiAgICAgIGNvbnN0IGpvdXJuYWxfZmlsZSA9IGAke2Z1bGxfcGF0aH0vJHtmaWxlcy5maWxlbmFtZX1gOwogICAgICBjb25zdCBkYXRhID0gZ2V0Sm91cm5hbChqb3VybmFsX2ZpbGUpOwogICAgICByZXR1cm4gZGF0YTsKICAgIH0KICB9Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("journal"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
