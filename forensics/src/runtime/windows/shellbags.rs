use crate::{
    artifacts::os::windows::shellbags::parser::grab_shellbags,
    runtime::helper::{boolean_arg, string_arg},
    structs::artifacts::os::windows::ShellbagsOptions,
};
use boa_engine::{Context, JsArgs, JsError, JsResult, JsValue, js_string};

/// Expose parsing shellbags to `BoaJS`
pub(crate) fn js_shellbags(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let resolve = boolean_arg(args, 0)?;
    let path = if args.get_or_undefined(1).is_undefined() {
        None
    } else {
        Some(string_arg(args, 1)?)
    };

    let options = ShellbagsOptions {
        alt_file: path,
        resolve_guids: resolve,
    };
    let bags = match grab_shellbags(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get shellbags: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&bags).unwrap_or_default();
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
    fn test_js_shellbags() {
        let test = "Ly8gZGVuby1mbXQtaWdub3JlLWZpbGUKLy8gZGVuby1saW50LWlnbm9yZS1maWxlCi8vIFRoaXMgY29kZSB3YXMgYnVuZGxlZCB1c2luZyBgZGVubyBidW5kbGVgIGFuZCBpdCdzIG5vdCByZWNvbW1lbmRlZCB0byBlZGl0IGl0IG1hbnVhbGx5CgpmdW5jdGlvbiBnZXRfc2hlbGxiYWdzKHJlc29sdmVfZ3VpZHMpIHsKICAgIGNvbnN0IGRhdGEgPSBqc19zaGVsbGJhZ3MocmVzb2x2ZV9ndWlkcyk7CiAgICByZXR1cm4gZGF0YTsKfQpmdW5jdGlvbiBnZXRTaGVsbGJhZ3MocmVzb2x2ZV9ndWlkcykgewogICAgcmV0dXJuIGdldF9zaGVsbGJhZ3MocmVzb2x2ZV9ndWlkcyk7Cn0KZnVuY3Rpb24gbWFpbigpIHsKICAgIGNvbnN0IGJhZ3MgPSBnZXRTaGVsbGJhZ3ModHJ1ZSk7CiAgICByZXR1cm4gYmFnczsKfQptYWluKCk7";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("shellbags"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
