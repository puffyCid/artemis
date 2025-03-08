use crate::{
    artifacts::os::windows::shimdb::parser::grab_shimdb, runtime::helper::string_arg,
    structs::artifacts::os::windows::ShimdbOptions,
};
use boa_engine::{Context, JsArgs, JsError, JsResult, JsValue, js_string};

/// Expose parsing shimdb located on systemdrive to `BoaJS`
pub(crate) fn js_shimdb(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = if args.get_or_undefined(0).is_undefined() {
        None
    } else {
        Some(string_arg(args, &1)?)
    };
    let options = ShimdbOptions { alt_file: path };
    let shimdb = match grab_shimdb(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get shimdb: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&shimdb).unwrap_or_default();
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
    fn test_js_shimdb() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfc2hpbWRiKCkgewogICAgY29uc3QgZGF0YSA9IGpzX3NoaW1kYigpOwogICAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24gZ2V0U2hpbWRiKCkgewogICAgcmV0dXJuIGdldF9zaGltZGIoKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3Qgc2RiID0gZ2V0U2hpbWRiKCk7CiAgICByZXR1cm4gc2RiOwp9Cm1haW4oKTsKCg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("shimdb"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
