use crate::{
    artifacts::os::windows::search::parser::grab_search_path,
    runtime::helper::{number_arg, string_arg},
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};

/// Expose parsing Windows Search to `BoaJS`
pub(crate) fn js_search(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;
    let page_limit = number_arg(args, 1)? as u32;

    let search = match grab_search_path(&path, page_limit) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to parse search {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&search).unwrap_or_default();
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
    fn test_js_search() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy9zZWFyY2gudHMKZnVuY3Rpb24gZ2V0U2VhcmNoKHBhdGgpIHsKICBjb25zdCBkYXRhPSBqc19zZWFyY2gocGF0aCwgNTApOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9lbnZpcm9ubWVudC9lbnYudHMKZnVuY3Rpb24gZ2V0RW52VmFsdWUoa2V5KSB7CiAgY29uc3QgZGF0YSA9IGpzX2Vudl92YWx1ZShrZXkpOwogIHJldHVybiBkYXRhOwp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9maWxlc3lzdGVtL2ZpbGVzLnRzCmZ1bmN0aW9uIHN0YXQocGF0aCkgewogIGNvbnN0IGRhdGEgPSBqc19zdGF0KHBhdGgpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZHJpdmUgPSBnZXRFbnZWYWx1ZSgiU3lzdGVtRHJpdmUiKTsKICBpZiAoZHJpdmUgPT09ICIiKSB7CiAgICByZXR1cm4gW107CiAgfQogIGNvbnN0IHBhdGggPSBgJHtkcml2ZX1cXFByb2dyYW1EYXRhXFxNaWNyb3NvZnRcXFNlYXJjaFxcRGF0YVxcQXBwbGljYXRpb25zXFxXaW5kb3dzYDsKICB0cnkgewogICAgY29uc3Qgc2VhcmNoX3BhdGggPSBgJHtwYXRofVxcV2luZG93cy5lZGJgOwogICAgY29uc3Qgc3RhdHVzID0gc3RhdChzZWFyY2hfcGF0aCk7CiAgICBpZiAoIXN0YXR1cy5pc19maWxlKSB7CiAgICAgIHJldHVybiBbXTsKICAgIH0KICAgIGNvbnN0IHJlc3VsdHMgPSBnZXRTZWFyY2goc2VhcmNoX3BhdGgpOwogICAgcmV0dXJuIHJlc3VsdHM7CiAgfSBjYXRjaCAoX2UpIHsKICAgIGNvbnN0IHNlYXJjaF9wYXRoID0gYCR7cGF0aH1cXFdpbmRvd3MuZGJgOwogICAgY29uc3Qgc3RhdHVzID0gc3RhdChzZWFyY2hfcGF0aCk7CiAgICBpZiAoIXN0YXR1cy5pc19maWxlKSB7CiAgICAgIHJldHVybiBbXTsKICAgIH0KICAgIGNvbnN0IHJlc3VsdHMgPSBnZXRTZWFyY2goc2VhcmNoX3BhdGgpOwogICAgcmV0dXJuIHJlc3VsdHM7CiAgfQp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("search"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
