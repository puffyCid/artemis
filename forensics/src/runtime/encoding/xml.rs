use crate::{
    runtime::helper::string_arg,
    utils::{encoding::read_xml, strings::extract_utf8_string},
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};
use nom::AsBytes;
use quick_xml::{Reader, XmlVersion, events::Event};
use serde_json::{Map, Value};

/// Read XML file into a JSON object
pub(crate) fn js_read_xml(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;

    // read_xml supports UTF16 and UTF8 encodings
    let xml = match read_xml(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not get read xml at '{path}': {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let xml_json = xml_to_json(&xml)?;
    let value = JsValue::from_json(&xml_json, context)?;

    Ok(value)
}

// Parse XML string into generic serde Value
fn xml_to_json(xml_string: &str) -> JsResult<Value> {
    let mut xml_reader = Reader::from_str(xml_string);
    let mut buf = Vec::new();

    let mut json_stack: Vec<(String, Map<String, Value>)> = Vec::new();
    let mut root_elements = Map::new();

    loop {
        buf.clear();

        let value = match xml_reader.read_event_into(&mut buf) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Could not get parse xml: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        match value {
            Event::Start(bytes_start) => {
                let tag_name = extract_utf8_string(bytes_start.name().0);
                let mut current_map = Map::new();
                for attr_value in bytes_start.attributes().flatten() {
                    let key = format!(
                        "@{}",
                        String::from_utf8(attr_value.key.0.to_vec()).unwrap_or_default()
                    );
                    let value = attr_value
                        .normalized_value(XmlVersion::Implicit1_0)
                        .map_err(|err| {
                            JsError::from_opaque(
                                js_string!(format!("Failed to normalize start attribute: {err:?}"))
                                    .into(),
                            )
                        })?;
                    current_map.insert(key, Value::String(value.into()));
                }
                json_stack.push((tag_name, current_map));
            }
            Event::End(bytes_end) => {
                let tag_name = extract_utf8_string(bytes_end.name().0);
                if let Some((popped_tag, mut popped_map)) = json_stack.pop() {
                    if popped_tag != tag_name {
                        let issue = format!("Got unexpected closing XML tag '{popped_tag}'");
                        return Err(JsError::from_opaque(js_string!(issue).into()));
                    }

                    let final_value = if popped_map.len() == 1 && popped_map.contains_key("#text") {
                        popped_map.remove("#text").unwrap_or_default()
                    } else if popped_map.is_empty() {
                        Value::String(String::new())
                    } else {
                        Value::Object(popped_map)
                    };

                    if let Some((_, parent_map)) = json_stack.last_mut() {
                        insert_into_json(parent_map, tag_name, final_value);
                    } else {
                        insert_into_json(&mut root_elements, tag_name, final_value);
                    }
                }
            }
            Event::Empty(bytes_start) => {
                let tag_name = extract_utf8_string(bytes_start.name().0);
                let mut current_map = Map::new();

                for attr_value in bytes_start.attributes().flatten() {
                    let key = format!("@{}", extract_utf8_string(attr_value.key.0));
                    let value = attr_value
                        .normalized_value(XmlVersion::Implicit1_0)
                        .map_err(|err| {
                            JsError::from_opaque(
                                js_string!(format!(
                                    "Failed to normalize empty element tag: {err:?}"
                                ))
                                .into(),
                            )
                        })?;

                    current_map.insert(key, Value::String(value.into()));
                }

                let final_value = if current_map.is_empty() {
                    Value::String(String::new())
                } else {
                    Value::Object(current_map)
                };

                if let Some((_, parent_map)) = json_stack.last_mut() {
                    insert_into_json(parent_map, tag_name, final_value);
                } else {
                    insert_into_json(&mut root_elements, tag_name, final_value);
                }
            }
            Event::Text(bytes_text) => {
                let text = extract_utf8_string(bytes_text.as_bytes());
                if text.is_empty() || text.trim_ascii().is_empty() {
                    continue;
                }

                if let Some((_, current_map)) = json_stack.last_mut() {
                    current_map.insert(String::from("#text"), Value::String(text));
                }
            }
            Event::CData(bytes_cdata) => {
                let text = extract_utf8_string(bytes_cdata.as_bytes());
                if let Some((_, current_map)) = json_stack.last_mut() {
                    current_map.insert(String::from("#text"), Value::String(text));
                }
            }
            Event::GeneralRef(bytes_general) => {
                let text = extract_utf8_string(bytes_general.as_bytes());
                if let Some((_, current_map)) = json_stack.last_mut() {
                    current_map.insert(String::from("#text"), Value::String(text));
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }

    Ok(Value::Object(root_elements))
}

/// Insert our value into the json object
fn insert_into_json(map: &mut Map<String, Value>, key: String, value: Value) {
    match map.get_mut(&key) {
        Some(Value::Array(array_value)) => array_value.push(value),
        Some(existing) => {
            let old = existing.take();
            map.insert(key, Value::Array(vec![old, value]));
        }
        None => {
            map.insert(key, value);
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::{
        output::manager::OutputManager, runtime::run::execute_script,
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
    #[cfg(target_os = "windows")]
    fn test_js_read_xml() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21haW4vc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gZ2xvYihwYXR0ZXJuKSB7CiAgY29uc3QgZGF0YSA9IGpzX2dsb2IocGF0dGVybik7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYWluL3NyYy9lbmNvZGluZy94bWwudHMKZnVuY3Rpb24gcmVhZFhtbChwYXRoKSB7CiAgY29uc3QgcmVzdWx0ID0ganNfcmVhZF94bWwocGF0aCk7CiAgcmV0dXJuIHJlc3VsdDsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGhzID0gZ2xvYigiQzpcXCpcXCoueG1sIik7CiAgaWYgKHBhdGhzIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYEZhaWxlZCB0byBnbG9iIHBhdGg6ICR7cGF0aHN9YCk7CiAgICByZXR1cm4gcGF0aHM7CiAgfQogIGZvciAoY29uc3QgZW50cnkgb2YgcGF0aHMpIHsKICAgIGlmICghZW50cnkuaXNfZmlsZSkgewogICAgICBjb250aW51ZTsKICAgIH0KICAgIGNvbnN0IHJldXNsdCA9IHJlYWRYbWwoZW50cnkuZnVsbF9wYXRoKTsKICAgIHJldHVybiByZXVzbHQ7CiAgfQp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "./tmp", false);
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
        let mut output = output_options("runtime_test", "./tmp", false);
        let script = JSScript {
            name: String::from("xml_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
