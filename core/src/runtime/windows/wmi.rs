use crate::{
    artifacts::os::windows::wmi::{
        helper::{class_description, get_pages, list_classes, list_namespaces},
        index::{IndexBody, parse_index},
        parser::grab_wmi_persist,
    },
    runtime::helper::{boolean_arg, bytes_arg, number_arg, string_arg, value_arg},
    structs::artifacts::os::windows::WmiPersistOptions,
};
use boa_engine::{Context, JsArgs, JsError, JsResult, JsValue, js_string};

/// Expose parsing wmi persist to `BoaJS`
pub(crate) fn js_wmipersist(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = if args.get_or_undefined(0).is_undefined() {
        None
    } else {
        Some(string_arg(args, &0)?)
    };
    let options = WmiPersistOptions { alt_dir: path };

    let wmi = match grab_wmi_persist(&options) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get wmipersist: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&wmi).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Returns list of namespaces or classes
pub(crate) fn js_list_namespaces_classes(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let namespace = string_arg(args, &0)?;
    let indexes_value = value_arg(args, &1, context)?;
    let object_data = bytes_arg(args, &2, context)?;
    let pages_value = value_arg(args, &3, context)?;
    let is_classes = boolean_arg(args, &4, context)?;

    let indexes: Vec<IndexBody> = match serde_json::from_value(indexes_value) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to deserialize WMI Indexes: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let pages: Vec<u32> = match serde_json::from_value(pages_value) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to deserialize WMI Pages: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    if !is_classes {
        let spaces = match list_namespaces(&namespace, &indexes, &object_data, &pages) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to get WMI namesapces: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

        let results = serde_json::to_value(&spaces).unwrap_or_default();
        let value = JsValue::from_json(&results, context)?;

        return Ok(value);
    }

    let classes = match list_classes(&namespace, &indexes, &object_data, &pages) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get WMI classes: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&classes).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Return descriptions for class
pub(crate) fn js_class_description(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let namespace = string_arg(args, &0)?;
    let locale = number_arg(args, &1)? as u32;
    let class_name = string_arg(args, &2)?;

    let indexes_value = value_arg(args, &3, context)?;
    let object_data = bytes_arg(args, &4, context)?;
    let pages_value = value_arg(args, &5, context)?;

    let indexes: Vec<IndexBody> = match serde_json::from_value(indexes_value) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to deserialize WMI Indexes for class description: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let pages: Vec<u32> = match serde_json::from_value(pages_value) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to deserialize WMI Pages for class description: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let desc = match class_description(
        &namespace,
        &locale,
        &class_name,
        &indexes,
        &object_data,
        &pages,
    ) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get WMI class description: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&desc).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Parse WMI pages
pub(crate) fn js_get_wmi_pages(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let map_path = string_arg(args, &0)?;

    let pages = match get_pages(&map_path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get WMI pages: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&pages).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Extract WMI index
pub(crate) fn js_get_wmi_indexes(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let data = bytes_arg(args, &0, context)?;
    let index = match parse_index(&data) {
        Ok((_, result)) => result,
        Err(err) => {
            let issue = format!("Failed to get WMI indexes: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&index).unwrap_or_default();
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
    fn test_js_wmipersist() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvZXJyb3JzLnRzCnZhciBXaW5kb3dzRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvZW52aXJvbm1lbnQvZW52LnRzCmZ1bmN0aW9uIGdldEVudlZhbHVlKGtleSkgewogIGNvbnN0IGRhdGEgPSBqc19lbnZfdmFsdWUoa2V5KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3Mvd21pLnRzCmZ1bmN0aW9uIGdldFdtaVBlcnNpc3QoKSB7CiAgdHJ5IHsKICAgIGNvbnN0IGRhdGEgPSBqc193bWlwZXJzaXN0KCk7CiAgICByZXR1cm4gZGF0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgV2luZG93c0Vycm9yKCJXTUlQRVJTSVNUIiwgYGZhaWxlZCB0byBwYXJzZSBXTUkgcmVwbzogJHtlcnJ9YCk7CiAgfQp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZGF0YSA9IGdldFdtaVBlcnNpc3QoKTsKICByZXR1cm4gZGF0YTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("wmipersist"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_js_wmi() {
        let test = "KCgpPT57dmFyIGE9Y2xhc3MgZXh0ZW5kcyBFcnJvcntuYW1lO21lc3NhZ2U7Y29uc3RydWN0b3IocixvKXtzdXBlcigpLHRoaXMubmFtZT1yLHRoaXMubWVzc2FnZT1vfX07dmFyIHQ9Y2xhc3MgZXh0ZW5kcyBhe307ZnVuY3Rpb24gcChlPSJyb290IixyLG8sbil7dHJ5e3JldHVybiBqc19saXN0X25hbWVzcGFjZXNfY2xhc3NlcyhlLHIsbyxuLCExKX1jYXRjaChzKXtyZXR1cm4gbmV3IHQoIldNSVBFUlNJU1QiLGBmYWlsZWQgdG8gbGlzdCBuYW1lc3BhY2VzOiAke3N9YCl9fWZ1bmN0aW9uIEkoZT0icm9vdCIscixvLG4pe3RyeXtyZXR1cm4ganNfbGlzdF9uYW1lc3BhY2VzX2NsYXNzZXMoZSxyLG8sbiwhMCl9Y2F0Y2gocyl7cmV0dXJuIG5ldyB0KCJXTUlQRVJTSVNUIixgZmFpbGVkIHRvIGxpc3QgY2xhc3NlczogJHtzfWApfX1mdW5jdGlvbiBTKGUscixvLG4scyxkKXt0cnl7cmV0dXJuIGpzX2NsYXNzX2Rlc2NyaXB0aW9uKGUscixvLG4scyxkKX1jYXRjaChpKXtyZXR1cm4gbmV3IHQoIldNSVBFUlNJU1QiLGBmYWlsZWQgdG8gZ2V0IGNsYXNzIGRlc2NyaXB0aW9uOiAke2l9YCl9fWZ1bmN0aW9uIGcoZSl7dHJ5e3JldHVybiBqc19nZXRfd21pX3BhZ2VzKGUpfWNhdGNoKHIpe3JldHVybiBuZXcgdCgiV01JUEVSU0lTVCIsYGZhaWxlZCB0byBnZXQgd21pIHBhZ2VzOiAke3J9YCl9fWZ1bmN0aW9uIHgoZSl7dHJ5e3JldHVybiBqc19nZXRfd21pX2luZGV4ZXMoZSl9Y2F0Y2gocil7cmV0dXJuIG5ldyB0KCJXTUlQRVJTSVNUIixgZmFpbGVkIHRvIGdldCB3bWkgaW5kZXhlczogJHtyfWApfX12YXIgYz1jbGFzcyBleHRlbmRzIGF7fTtmdW5jdGlvbiBsKGUpe3RyeXtyZXR1cm4ganNfcmVhZF9maWxlKGUpfWNhdGNoKHIpe3JldHVybiBuZXcgYygiUkVBRF9GSUxFIixgZmFpbGVkIHRvIHJlYWQgZmlsZSAke2V9OiAke3J9YCl9fWZ1bmN0aW9uIFQoKXtsZXQgZT0iQzpcXFdpbmRvd3NcXFN5c3RlbTMyXFx3YmVtXFxSZXBvc2l0b3J5XFxNQVBQSU5HKi5NQVAiLHI9IkM6XFxXaW5kb3dzXFxTeXN0ZW0zMlxcd2JlbVxcUmVwb3NpdG9yeVxcSU5ERVguQlRSIixvPSJDOlxcV2luZG93c1xcU3lzdGVtMzJcXHdiZW1cXFJlcG9zaXRvcnlcXE9CSkVDVFMuREFUQSIsbj1nKGUpLHM9bChvKSxkPWwociksaT14KGQpLEU9cCgicm9vdCIsaSxzLG4pO2lmKEUgaW5zdGFuY2VvZiB0KXJldHVybjtmb3IobGV0IG0gb2YgRSlpZihjb25zb2xlLmxvZyhgTmFtZXNwYWNlOiAke20ucGF0aH1gKSxtLm5hbWUuaW5jbHVkZXMoImNpbXYyIikpe2xldCB1PUkobS5wYXRoLGkscyxuKTtpZih1IGluc3RhbmNlb2YgdCljb250aW51ZTtjb25zb2xlLmxvZyhgRm91bmQgJHt1Lmxlbmd0aH0gZm9yIENJTXYyYCl9bGV0IGY9Uygicm9vdFxcY2ltdjIiLDEwMzMsIldpbjMyX0JJT1MiLGkscyxuKTtmIGluc3RhbmNlb2YgdHx8Y29uc29sZS5sb2coSlNPTi5zdHJpbmdpZnkoZi5wcm9wZXJ0aWVzKSl9VCgpO30pKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("wmipersist"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
