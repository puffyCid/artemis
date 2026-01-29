use crate::{
    artifacts::os::windows::usnjrnl::parser::grab_usnjrnl,
    runtime::helper::{char_arg, string_arg},
    structs::artifacts::os::windows::UsnJrnlOptions,
};
use boa_engine::{Context, JsArgs, JsError, JsResult, JsValue, js_string};

/// Expose parsing usnjrnl located on systemdrive to `BoaJS`
pub(crate) fn js_usnjrnl(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = if args.get_or_undefined(0).is_undefined() {
        None
    } else {
        Some(string_arg(args, 0)?)
    };
    let drive = if args.get_or_undefined(1).is_undefined() {
        None
    } else {
        Some(char_arg(args, 1)?)
    };
    let mft_path = if args.get_or_undefined(2).is_undefined() {
        None
    } else {
        Some(string_arg(args, 2)?)
    };

    let options = UsnJrnlOptions {
        alt_drive: drive,
        alt_path: path,
        alt_mft: mft_path,
    };
    let jrnl = match grab_usnjrnl(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get usnjrnl: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&jrnl).unwrap_or_default();
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
    fn test_js_usnjrnl_rs_files() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfdXNuanJubCgpIHsKdHJ5IHsKICAgIGNvbnN0IGRhdGEgPSBqc191c25qcm5sKCk7CiAgICByZXR1cm4gZGF0YTsKfWNhdGNoKGVycikge3JldHVybiBlcnI7fQp9CmZ1bmN0aW9uIGdldFVzbkpybmwoKSB7CiAgICByZXR1cm4gZ2V0X3VzbmpybmwoKTsKfQpmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QganJubF9lbnRyaWVzID0gZ2V0VXNuSnJubCgpOwogICAgY29uc3QgcnNfZW50cmllcyA9IFtdOwogICAgZm9yKGxldCBlbnRyeSA9IDA7IGVudHJ5IDwganJubF9lbnRyaWVzLmxlbmd0aDsgZW50cnkrKyl7CiAgICAgICAgaWYgKGpybmxfZW50cmllc1tlbnRyeV0uZXh0ZW5zaW9uID09PSAicnMiKSB7CiAgICAgICAgICAgIHJzX2VudHJpZXMucHVzaChqcm5sX2VudHJpZXNbZW50cnldKTsKICAgICAgICAgICAgaWYocnNfZW50cmllcy5sZW5ndGggPiAxMDApewogICAgICAgICAgICAgIGJyZWFrOwogICAgICAgICAgICB9CiAgICAgICAgfQogICAgfQogICAgcmV0dXJuIHJzX2VudHJpZXM7Cn0KbWFpbigpOwoK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("usnjnl_rs_files"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
