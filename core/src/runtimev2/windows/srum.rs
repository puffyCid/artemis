use crate::{artifacts::os::windows::srum::parser::grab_srum_path, runtimev2::helper::string_arg};
use boa_engine::{js_string, Context, JsError, JsResult, JsValue};

/// Expose parsing a single SRUM table to `BoaJS`
pub(crate) fn js_srum(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let table = string_arg(args, &1)?;

    let srum = match grab_srum_path(&path, &table) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get srum: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let value = JsValue::from_json(&srum, context)?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtimev2::run::execute_script,
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
    fn test_js_srum() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfc3J1bV9hcHBsaWNhdGlvbl9pbmZvKHBhdGgpIHsKICAgIGNvbnN0IG5hbWUgPSAie0QxMENBMkZFLTZGQ0YtNEY2RC04NDhFLUIyRTk5MjY2RkE4OX0iOwogICAgY29uc3QgZGF0YSA9IGpzX3NydW0ocGF0aCwgbmFtZSk7CiAgICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBnZXRTcnVtQXBwbGljYXRpb25JbmZvKHBhdGgpIHsKICAgIHJldHVybiBnZXRfc3J1bV9hcHBsaWNhdGlvbl9pbmZvKHBhdGgpOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBwYXRoID0gIkM6XFxXaW5kb3dzXFxTeXN0ZW0zMlxcc3J1XFxTUlVEQi5kYXQiOwogICAgY29uc3QgZW50cmllcyA9IGdldFNydW1BcHBsaWNhdGlvbkluZm8ocGF0aCk7CiAgICByZXR1cm4gZW50cmllczsKfQptYWluKCk7Cgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("srum"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
