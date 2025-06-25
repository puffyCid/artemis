use crate::{
    artifacts::os::windows::amcache::parser::grab_amcache, runtime::helper::string_arg,
    structs::artifacts::os::windows::AmcacheOptions,
};
use boa_engine::{Context, JsArgs, JsError, JsResult, JsValue, js_string};

/// Expose parsing amcache `BoaJS`
pub(crate) fn js_amcache(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = if args.get_or_undefined(0).is_undefined() {
        None
    } else {
        Some(string_arg(args, 0)?)
    };

    let options = AmcacheOptions { alt_file: path };
    let amcache = match grab_amcache(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get amcache: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&amcache).unwrap_or_default();
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
    fn test_js_amcache() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfYW1jYWNoZSgpIHsKdHJ5IHsKICAgIGNvbnN0IGRhdGEgPSBqc19hbWNhY2hlKCk7CiAgICByZXR1cm4gZGF0YTsKfWNhdGNoIChlcnIpe3JldHVybiBlcnI7fQp9CmZ1bmN0aW9uIGdldEFtY2FjaGUoKSB7CiAgICByZXR1cm4gZ2V0X2FtY2FjaGUoKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgY2FjaGUgPSBnZXRBbWNhY2hlKCk7CiAgICByZXR1cm4gY2FjaGU7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("amcache"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
