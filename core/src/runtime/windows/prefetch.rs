use crate::{
    artifacts::os::windows::prefetch::parser::grab_prefetch, runtime::helper::string_arg,
    structs::artifacts::os::windows::PrefetchOptions,
};
use boa_engine::{js_string, Context, JsArgs, JsError, JsResult, JsValue};

/// Expose parsing prefetch to `BoaJS`
pub(crate) fn js_prefetch(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = if args.get_or_undefined(0).is_undefined() {
        None
    } else {
        Some(string_arg(args, &0)?)
    };

    let options = PrefetchOptions { alt_dir: path };
    let pf = match grab_prefetch(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to parse prefetch: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&pf).unwrap_or_default();
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
    fn test_js_prefetch() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfcHJlZmV0Y2goKSB7CiAgICBjb25zdCBkYXRhID0ganNfcHJlZmV0Y2goKTsKICAgIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIGdldFByZWZldGNoKCkgewogICAgcmV0dXJuIGdldF9wcmVmZXRjaCgpOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBwZiA9IGdldFByZWZldGNoKCk7CiAgICByZXR1cm4gcGY7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("pf_default"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
