use crate::{
    artifacts::os::windows::jumplists::parser::grab_jumplists, runtime::helper::string_arg,
    structs::artifacts::os::windows::JumplistsOptions,
};
use boa_engine::{js_string, Context, JsArgs, JsError, JsResult, JsValue};

/// Expose parsing Jumplists to `BoajS`
pub(crate) fn js_jumplists(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = if args.get_or_undefined(0).is_undefined() {
        None
    } else {
        Some(string_arg(args, &0)?)
    };
    let options = JumplistsOptions { alt_file: path };
    let jumplist = match grab_jumplists(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get jumplists: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&jumplist).unwrap_or_default();
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
    fn test_js_jumplists() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9qdW1wbGlzdHMudHMKZnVuY3Rpb24gZ2V0SnVtcGxpc3RzKCkgewogIGNvbnN0IGRhdGEgPSBqc19qdW1wbGlzdHMoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGp1bXAgPSBnZXRKdW1wbGlzdHMoKTsKICByZXR1cm4ganVtcDsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("jumplist_default"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
