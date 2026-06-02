use crate::{
    artifacts::os::windows::shimcache::parser::grab_shimcache, runtime::helper::string_arg,
    structs::artifacts::os::windows::ShimcacheOptions,
};
use boa_engine::{Context, JsArgs, JsError, JsResult, JsValue, js_string};

/// Expose parsing shimcache located on default drive to `BoaJS`
pub(crate) fn js_shimcache(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = if args.get_or_undefined(0).is_undefined() {
        None
    } else {
        Some(string_arg(args, 0)?)
    };
    let options = ShimcacheOptions { alt_file: path };
    let shim = match grab_shimcache(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get shimcache: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&shim).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::{
        output2::{
            config::{OutputConfig, OutputDestination, OutputFormat},
            manager::OutputManager,
        },
        runtime::run::execute_script,
        structs::artifacts::runtime::script::JSScript,
    };
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_js_shimcache() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfc2hpbWNhY2hlKCkgewogICAgY29uc3QgZGF0YSA9IGpzX3NoaW1jYWNoZSgpOwogICAgcmV0dXJuIGRhdGE7Cn0KZnVuY3Rpb24gZ2V0U2hpbWNhY2hlKCkgewogICAgcmV0dXJuIGdldF9zaGltY2FjaGUoKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3Qgc2hpbWNhY2hlX2VudHJpZXMgPSBnZXRTaGltY2FjaGUoKTsKICAgIHJldHVybiBzaGltY2FjaGVfZW50cmllczsKfQptYWluKCk7";
        let mut output = output_options("runtime_test", "./tmp", false);
        let script = JSScript {
            name: String::from("shimcache"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
