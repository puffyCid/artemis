use crate::{
    artifacts::os::processes::process::proc_list_entries,
    runtime::helper::{boolean_arg, value_arg},
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};
use common::files::Hashes;

/// Expose pulling process listing to `BoaJS`
pub(crate) fn js_get_processes(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let input = value_arg(args, 0, context)?;
    let metadata = boolean_arg(args, 1, context)?;

    let hashes: Hashes = serde_json::from_value(input).unwrap_or(Hashes {
        md5: false,
        sha1: false,
        sha256: false,
    });
    let proc = match proc_list_entries(&hashes, metadata) {
        Ok(results) => results,
        Err(err) => {
            let issue = format!("Failed to get process listing: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&proc).unwrap_or_default();
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
    fn test_js_get_processes() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfcHJvY2Vzc2VzKG1kNSwgc2hhMSwgc2hhMjU2LCBwZV9pbmZvKSB7CiAgICBjb25zdCBoYXNoZXMgPSB7CiAgICAgICAgbWQ1LAogICAgICAgIHNoYTEsCiAgICAgICAgc2hhMjU2CiAgICB9OwogICAgY29uc3QgZGF0YSA9IGpzX2dldF9wcm9jZXNzZXMoaGFzaGVzLCBwZV9pbmZvKTsKICAgIHJldHVybiBkYXRhOwp9CmZ1bmN0aW9uIGdldFByb2Nlc3NlcyhtZDUsIHNoYTEsIHNoYTI1NiwgcGVfaW5mbykgewogICAgcmV0dXJuIGdldF9wcm9jZXNzZXMobWQ1LCBzaGExLCBzaGEyNTYsIHBlX2luZm8pOwp9CmZ1bmN0aW9uIG1haW4oKSB7CiAgICBjb25zdCBwcm9jX2xpc3QgPSBnZXRQcm9jZXNzZXMoZmFsc2UsIGZhbHNlLCBmYWxzZSwgdHJ1ZSk7CiAgICByZXR1cm4gcHJvY19saXN0Owp9Cm1haW4oKTs=";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("processes"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
