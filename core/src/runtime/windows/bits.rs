use crate::{
    artifacts::os::windows::bits::parser::grab_bits,
    runtime::helper::{boolean_arg, string_arg},
    structs::artifacts::os::windows::BitsOptions,
};
use boa_engine::{Context, JsArgs, JsError, JsResult, JsValue, js_string};

/// Expose parsing BITS to `BoaJS`
pub(crate) fn js_bits(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let carve = boolean_arg(args, &0, context)?;
    let path = if args.get_or_undefined(1).is_undefined() {
        None
    } else {
        Some(string_arg(args, &1)?)
    };

    let options = BitsOptions {
        alt_file: path,
        carve,
    };
    let bits = match grab_bits(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get bits: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&bits).unwrap_or_default();
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
    fn test_js_bits() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfYml0cyhjYXJ2ZSkgewp0cnkgewogICAgY29uc3QgZGF0YSA9IGpzX2JpdHMoY2FydmUpOwogICAgcmV0dXJuIGRhdGE7Cn1jYXRjaChlcnIpe3JldHVybiBlcnI7fQp9CmZ1bmN0aW9uIGdldEJpdHMoY2FydmUpIHsKICAgIHJldHVybiBnZXRfYml0cyhjYXJ2ZSk7Cn0KZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IGVudHJpZXMgPSBnZXRCaXRzKHRydWUpOwogICAgcmV0dXJuIGVudHJpZXM7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("bits"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
