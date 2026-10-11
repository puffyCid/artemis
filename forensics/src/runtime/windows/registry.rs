use crate::{
    artifacts::os::windows::registry::helper::{get_registry_keys_handle, lookup_sk_info},
    runtime::{
        filesystem::accessor::return_file_handle_object,
        helper::{number_arg, string_arg, value_arg},
    },
    utils::regex_options::create_regex,
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};

/// Expose parsing Registry file to `BoaJS`
pub(crate) fn js_registry(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let value = value_arg(args, 0, context)?;
    let handle = return_file_handle_object(value)?;

    let key_regex = match create_regex(&string_arg(args, 1)?) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to setup Regex: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let start_root = string_arg(args, 3).unwrap_or_default();
    let reg = match get_registry_keys_handle(start_root, key_regex, &handle) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to parse registry: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&reg).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Expose parsing the Security Key to `BoaJS`
pub(crate) fn js_sk_info(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;
    let offset = number_arg(args, 1)? as i32;

    let sk = match lookup_sk_info(&path, offset) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to parse security key info: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&sk).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::utils::encoding::base64_encode_standard;
    use crate::{
        output::manager::OutputManager, runtime::run::execute_script,
        structs::artifacts::runtime::script::JSScript,
    };
    use std::fs;
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
    fn test_js_registry() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/runtime/registry/main.js");

        let bytes = fs::read(test_location).unwrap();
        let test = base64_encode_standard(&bytes);

        let mut output = output_options("reg_runtime_test", "./tmp", false);
        let script = JSScript {
            name: String::from("js_registry_firewall"),
            script: test,
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_sk_info() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfcmVnaXN0cnkocGF0aCkgewogICAgY29uc3QgZGF0YSA9IGpzX3JlZ2lzdHJ5KHBhdGgpOwogICAgcmV0dXJuIGRhdGE7Cn0KCmZ1bmN0aW9uIGxvb2t1cFNlY3VyaXR5S2V5KHBhdGgsIG9mZnNldCkgewogICAgaWYgKG9mZnNldCA8PSAwKSB7CiAgICAgIHJldHVybiBuZXcgRXJyb3IoIkNhbm5vdCB1c2UgbmVnYXRpdmUgb2Zmc2V0IG9yIHplcm8hIik7CiAgICB9CiAgICAgIC8vQHRzLWlnbm9yZTogQ3VzdG9tIEFydGVtaXMgZnVuY3Rpb24KICAgICAgY29uc3QgZGF0YSA9IGpzX3NrX2luZm8ocGF0aCwgb2Zmc2V0KTsKICAKICAgICAgcmV0dXJuIGRhdGE7CiAgfQoKZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IHBhdGggPSAiQzpcXFdpbmRvd3NcXFN5c3RlbTMyXFxjb25maWdcXERSSVZFUlMiOwogICAgY29uc3QgcmVnID0gZ2V0X3JlZ2lzdHJ5KHBhdGgpOwogICAgZm9yIChjb25zdCBlbnRyaWVzIG9mIHJlZyl7CiAgICAgICAgcmV0dXJuIGxvb2t1cFNlY3VyaXR5S2V5KHBhdGgsIGVudHJpZXMuc2VjdXJpdHlfb2Zmc2V0KTsKICAgIH0KfQptYWluKCk7";
        let mut output = output_options("runtime_test", "./tmp", false);
        let script = JSScript {
            name: String::from("sk"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
