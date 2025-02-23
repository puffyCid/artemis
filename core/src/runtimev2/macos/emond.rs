use crate::{
    artifacts::os::macos::emond::parser::grab_emond, runtimev2::helper::string_arg,
    structs::artifacts::os::macos::EmondOptions,
};
use boa_engine::{js_string, Context, JsArgs, JsError, JsResult, JsValue};

/// Expose parsing Emond to `BoaJS`
pub(crate) fn js_emond(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = if args.get_or_undefined(0).is_undefined() {
        None
    } else {
        Some(string_arg(args, &0)?)
    };

    let options = EmondOptions { alt_path: path };

    let emond = match grab_emond(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get emond: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&emond).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

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
    fn test_get_emond() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfZW1vbmQoKSB7CiAgICBjb25zdCBkYXRhID0ganNfZW1vbmQoKTsKICAgIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIGdldEVtb25kKCkgewogICAgcmV0dXJuIGdldF9lbW9uZCgpOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBkYXRhID0gZ2V0RW1vbmQoKTsKICAgIHJldHVybiBkYXRhOwp9Cm1haW4oKTsKCg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("emond"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
