use crate::{
    artifacts::os::windows::eventlogs::parser::parse_eventlogs,
    runtime::helper::{boolean_arg, number_arg, string_arg},
};
use boa_engine::{Context, JsArgs, JsError, JsResult, JsValue, js_string};

/// Expose parsing a single eventlog file (evtx) to `BoaJS`
pub(crate) fn js_eventlogs(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, &0)?;
    let offset = number_arg(args, &1)? as usize;
    let limit = number_arg(args, &2)? as usize;
    let include_templates = boolean_arg(args, &3, context)?;

    let temp_option = if args.get_or_undefined(4).is_undefined() {
        None
    } else {
        Some(string_arg(args, &4)?)
    };

    let logs = match parse_eventlogs(
        &path,
        &(offset as usize),
        &(limit as usize),
        &include_templates,
        &temp_option,
    ) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get eventlogs: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&logs).unwrap_or_default();
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
    fn test_js_eventlogs() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvZXJyb3JzLnRzCnZhciBXaW5kb3dzRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvc3lzdGVtL3N5c3RlbWluZm8udHMKZnVuY3Rpb24gcGxhdGZvcm0oKSB7CiAgY29uc3QgZGF0YSA9IGpzX3BsYXRmb3JtKCk7CiAgcmV0dXJuIGRhdGE7Cn0KCi8vIC4uLy4uL1Byb2plY3RzL2FydGVtaXMtYXBpL3NyYy93aW5kb3dzL2V2ZW50bG9ncy50cwpmdW5jdGlvbiBnZXRFdmVudGxvZ3MocGF0aCwgb2Zmc2V0LCBsaW1pdCwgaW5jbHVkZV90ZW1wbGF0ZXMgPSBmYWxzZSwgdGVtcGxhdGVfZmlsZSA9IHVuZGVmaW5lZCkgewogIGlmIChpbmNsdWRlX3RlbXBsYXRlcyAmJiBwbGF0Zm9ybSgpICE9ICJXaW5kb3dzIiAmJiB0ZW1wbGF0ZV9maWxlID09IHVuZGVmaW5lZCkgewogICAgcmV0dXJuIG5ldyBXaW5kb3dzRXJyb3IoCiAgICAgICJFVkVOVExPRyIsCiAgICAgIGBjYW5ub3QgaW5jbHVkZSB0ZW1wbGF0ZSBzdHJpbmdzIG9uIG5vbi1XaW5kb3dzIHBsYXRmb3JtIHdpdGhvdXQgYSB0ZW1wbGF0ZSBmaWxlYAogICAgKTsKICB9CiAgdHJ5IHsKICAgIGNvbnN0IHJlc3VsdHMgPSBqc19ldmVudGxvZ3MoCiAgICAgIHBhdGgsCiAgICAgIG9mZnNldCwKICAgICAgbGltaXQsCiAgICAgIGluY2x1ZGVfdGVtcGxhdGVzLAogICAgICB0ZW1wbGF0ZV9maWxlCiAgICApOwogICAgcmV0dXJuIHJlc3VsdHM7CiAgfSBjYXRjaCAoZXJyKSB7CiAgICByZXR1cm4gbmV3IFdpbmRvd3NFcnJvcigKICAgICAgIkVWRU5UTE9HIiwKICAgICAgYGZhaWxlZCB0byBwYXJzZSBldmVudGxvZyAke3BhdGh9OiAke2Vycn1gCiAgICApOwogIH0KfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHBhdGggPSAiQzpcXFdpbmRvd3NcXFN5c3RlbTMyXFx3aW5ldnRcXExvZ3NcXEFwcGxpY2F0aW9uLmV2dHgiOwogIGNvbnN0IGxvZ3MgPSBnZXRFdmVudGxvZ3MoCiAgICBwYXRoLAogICAgMTAsCiAgICAxMCwKICAgIHRydWUKICApOwogIGlmIChsb2dzIGluc3RhbmNlb2YgV2luZG93c0Vycm9yKSB7CiAgICBjb25zb2xlLmVycm9yKGxvZ3MpOwogICAgcmV0dXJuOwogIH0KICBjb25zdCBtZXNzYWdlcyA9IGxvZ3NbMF07CiAgY29uc3QgcmF3ID0gbG9nc1sxXTsKICBjb25zb2xlLmxvZyhKU09OLnN0cmluZ2lmeShtZXNzYWdlcykpOwogIGNvbnNvbGUubG9nKEpTT04uc3RyaW5naWZ5KHJhdykpOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("service_installs"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
