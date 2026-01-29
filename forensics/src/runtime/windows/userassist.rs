use crate::{
    artifacts::os::windows::userassist::parser::grab_userassist,
    runtime::helper::{boolean_arg, string_arg},
    structs::artifacts::os::windows::UserAssistOptions,
};
use boa_engine::{Context, JsArgs, JsError, JsResult, JsValue, js_string};

/// Expose parsing userassist located on systemdrive to `BoaJS`
pub(crate) fn js_userassist(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let resolve = boolean_arg(args, 0)?;
    let path = if args.get_or_undefined(1).is_undefined() {
        None
    } else {
        Some(string_arg(args, 1)?)
    };
    let options = UserAssistOptions {
        alt_file: path,
        resolve_descriptions: Some(resolve),
    };

    let assist = match grab_userassist(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get userassist: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&assist).unwrap_or_default();
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
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_js_userassist() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfdXNlcmFzc2lzdCgpIHsKdHJ5IHsKICAgIGNvbnN0IGRhdGEgPSBqc191c2VyYXNzaXN0KGZhbHNlKTsKICAgIHJldHVybiBkYXRhOwogICAgfWNhdGNoKGVycil7cmV0dXJuIGVycjt9Cn0KZnVuY3Rpb24gZ2V0VXNlckFzc2lzdCgpIHsKICAgIHJldHVybiBnZXRfdXNlcmFzc2lzdCgpOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBhc3Npc3QgPSBnZXRVc2VyQXNzaXN0KCk7CiAgICByZXR1cm4gYXNzaXN0Owp9Cm1haW4oKTsKCg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("userassist"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
