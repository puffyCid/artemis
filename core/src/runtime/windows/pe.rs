use crate::{artifacts::os::windows::pe::parser::parse_pe_file, runtime::helper::string_arg};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};

/// Expose parsing pe file  to `BoaJS`
pub(crate) fn js_get_pe(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;

    let pe = match parse_pe_file(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to parse PE {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(pe).unwrap_or_default();
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
    fn test_get_pe() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9wZS50cwpmdW5jdGlvbiBnZXRQZShwYXRoKSB7CiAgY29uc3QgZGF0YSA9IGpzX2dldF9wZShwYXRoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW52aXJvbm1lbnQvZW52LnRzCmZ1bmN0aW9uIGdldEVudlZhbHVlKGtleSkgewogIGNvbnN0IGRhdGEgPSBqc19lbnZfdmFsdWUoa2V5KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwphc3luYyBmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGRyaXZlID0gZ2V0RW52VmFsdWUoIlN5c3RlbURyaXZlIik7CiAgaWYgKGRyaXZlID09PSAiIikgewogICAgcmV0dXJuIFtdOwogIH0KICBjb25zdCBwYXRoID0gYCR7ZHJpdmV9XFxXaW5kb3dzXFxleHBsb3Jlci5leGVgOwogIGNvbnN0IGluZm8gPSBnZXRQZShwYXRoKTsKCiAgcmV0dXJuIGluZm87Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("system32_pe"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
