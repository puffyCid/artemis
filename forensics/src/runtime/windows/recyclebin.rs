use crate::{
    artifacts::os::windows::recyclebin::parser::grab_recycle_bin, runtime::helper::string_arg,
    structs::artifacts::os::windows::RecycleBinOptions,
};
use boa_engine::{Context, JsArgs, JsError, JsResult, JsValue, js_string};

/// Expose parsing Recycle Bin to `BoaJS`
pub(crate) fn js_recycle_bin(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = if args.get_or_undefined(0).is_undefined() {
        None
    } else {
        Some(string_arg(args, 0)?)
    };

    let options = RecycleBinOptions { alt_file: path };
    let bin = match grab_recycle_bin(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to parse recyclebin: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&bin).unwrap_or_default();
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
    fn test_js_recycle_bin() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9yZWN5Y2xlYmluLnRzCmZ1bmN0aW9uIGdldFJlY3ljbGVCaW4oZHJpdmUpIHsKICAgIGNvbnN0IGRhdGEyID0ganNfcmVjeWNsZV9iaW4oKTsKICAgIHJldHVybiBkYXRhMjsgIAp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgYmluID0gZ2V0UmVjeWNsZUJpbigpOwogIHJldHVybiBiaW47Cn0KbWFpbigpOw==";
        let mut output = output_options("runtime_test", "./tmp", false);
        let script = JSScript {
            name: String::from("recycle_bin_default"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
