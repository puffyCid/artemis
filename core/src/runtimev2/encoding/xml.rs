use crate::{runtimev2::helper::string_arg, utils::encoding::read_xml};
use boa_engine::{js_string, Context, JsError, JsResult, JsValue};
use xml2json_rs::JsonBuilder;

/// Read XML file into a JSON object
pub(crate) fn js_read_xml(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;

    // read_xml supports UTF16 and UTF8 encodings
    let xml = match read_xml(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not get read xml {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    // Parse XML string into generic serde Value
    let xml_builder = JsonBuilder::default();
    let xml_json = match xml_builder.build_from_xml(&xml) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not parse xml {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let value = JsValue::from_json(&xml_json, context)?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::runtimev2::run::execute_script;
    use crate::{structs::artifacts::runtime::script::JSScript, structs::toml::Output};

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
    #[cfg(target_os = "windows")]
    fn test_js_read_xml() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21haW4vc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gZ2xvYihwYXR0ZXJuKSB7CiAgY29uc3QgZGF0YSA9IGpzX2dsb2IocGF0dGVybik7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYWluL3NyYy9lbmNvZGluZy94bWwudHMKZnVuY3Rpb24gcmVhZFhtbChwYXRoKSB7CiAgY29uc3QgcmVzdWx0ID0ganNfcmVhZF94bWwocGF0aCk7CiAgcmV0dXJuIHJlc3VsdDsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGhzID0gZ2xvYigiQzpcXCpcXCoueG1sIik7CiAgaWYgKHBhdGhzIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYEZhaWxlZCB0byBnbG9iIHBhdGg6ICR7cGF0aHN9YCk7CiAgICByZXR1cm4gcGF0aHM7CiAgfQogIGZvciAoY29uc3QgZW50cnkgb2YgcGF0aHMpIHsKICAgIGlmICghZW50cnkuaXNfZmlsZSkgewogICAgICBjb250aW51ZTsKICAgIH0KICAgIGNvbnN0IHJldXNsdCA9IHJlYWRYbWwoZW50cnkuZnVsbF9wYXRoKTsKICAgIHJldHVybiByZXVzbHQ7CiAgfQp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("xml_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    #[cfg(target_family = "unix")]
    fn test_js_read_xml() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21haW4vc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gZ2xvYihwYXR0ZXJuKSB7CiAgY29uc3QgZGF0YSA9IGpzX2dsb2IocGF0dGVybik7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYWluL3NyYy9lbmNvZGluZy94bWwudHMKZnVuY3Rpb24gcmVhZFhtbChwYXRoKSB7CiAgY29uc3QgcmVzdWx0ID0ganNfcmVhZF94bWwocGF0aCk7CiAgcmV0dXJuIHJlc3VsdDsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGhzID0gZ2xvYigiLyovKi54bWwiKTsKICBpZiAocGF0aHMgaW5zdGFuY2VvZiBFcnJvcikgewogICAgY29uc29sZS5lcnJvcihgRmFpbGVkIHRvIGdsb2IgcGF0aDogJHtwYXRoc31gKTsKICAgIHJldHVybiBwYXRoczsKICB9CiAgZm9yIChjb25zdCBlbnRyeSBvZiBwYXRocykgewogICAgaWYgKCFlbnRyeS5pc19maWxlKSB7CiAgICAgIGNvbnRpbnVlOwogICAgfQogICAgY29uc3QgcmV1c2x0ID0gcmVhZFhtbChlbnRyeS5mdWxsX3BhdGgpOwogICAgcmV0dXJuIHJldXNsdDsKICB9Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("xml_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
