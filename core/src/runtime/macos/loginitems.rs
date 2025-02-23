use crate::{
    artifacts::os::macos::loginitems::parser::grab_loginitems, runtime::helper::string_arg,
    structs::artifacts::os::macos::LoginitemsOptions,
};
use boa_engine::{js_string, Context, JsArgs, JsError, JsResult, JsValue};

/// Expose parsing `LoginItems` to `BoaJS`
pub(crate) fn js_loginitems(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = if args.get_or_undefined(0).is_undefined() {
        None
    } else {
        Some(string_arg(args, &0)?)
    };

    let options = LoginitemsOptions { alt_file: path };
    let loginitems = match grab_loginitems(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get loginitems: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&loginitems).unwrap_or_default();
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
    fn test_js_loginitems() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfbG9naW5pdGVtcygpIHsKICAgIGNvbnN0IGRhdGEgPSBqc19sb2dpbml0ZW1zKCk7CiAgICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBnZXRMb2dpbkl0ZW1zKCkgewogICAgcmV0dXJuIGdldF9sb2dpbml0ZW1zKCk7Cn0KZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IGRhdGEgPSBnZXRMb2dpbkl0ZW1zKCk7CiAgICByZXR1cm4gZGF0YTsKfQptYWluKCk7Cgo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("loginitems"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
