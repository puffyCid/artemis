use crate::{artifacts::os::windows::tasks::parser::grab_task_xml, runtime::helper::string_arg};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};

/// Expose parsing Schedule Tasks to `BoaJS`
pub(crate) fn js_tasks(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;
    let task = match grab_task_xml(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to get tasks: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&task).unwrap_or_default();
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
    fn test_js_tasks() {
        let test = "KCgpID0+IHsNCiAgLy8gLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzDQogIHZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsNCiAgICBuYW1lOw0KICAgIG1lc3NhZ2U7DQogICAgY29uc3RydWN0b3IobmFtZSwgbWVzc2FnZSkgew0KICAgICAgc3VwZXIoKTsNCiAgICAgIHRoaXMubmFtZSA9IG5hbWU7DQogICAgICB0aGlzLm1lc3NhZ2UgPSBtZXNzYWdlOw0KICAgIH0NCiAgfTsNCg0KICAvLyAuLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvd2luZG93cy9lcnJvcnMudHMNCiAgdmFyIFdpbmRvd3NFcnJvciA9IGNsYXNzIGV4dGVuZHMgRXJyb3JCYXNlIHsNCiAgfTsNCg0KICAvLyAuLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvd2luZG93cy90YXNrcy50cw0KICBmdW5jdGlvbiBnZXRUYXNrcyhwYXRoKSB7DQogICAgdHJ5IHsNCiAgICAgIGNvbnN0IGRhdGEgPSBqc190YXNrcyhwYXRoKTsNCiAgICAgIHJldHVybiBkYXRhOw0KICAgIH0gY2F0Y2ggKGVycikgew0KICAgICAgcmV0dXJuIG5ldyBXaW5kb3dzRXJyb3IoIlRBU0tTIiwgYGZhaWxlZCB0byBwYXJzZSB0YXNrczogJHtlcnJ9YCk7DQogICAgfQ0KICB9DQoNCiAgLy8gbWFpbi50cw0KICBmdW5jdGlvbiBtYWluKCkgew0KICAgIGNvbnN0IHJlc3VsdHMgPSBnZXRUYXNrcygiQzpcXFdpbmRvd3NcXFN5c3RlbTMyXFxUYXNrc1xcTWljcm9zb2Z0XFxXaW5kb3dzXFxEaXNrQ2xlYW51cFxcU2lsZW50Q2xlYW51cCIpOw0KICAgIGNvbnNvbGUubG9nKEpTT04uc3RyaW5naWZ5KHJlc3VsdHMpKTsNCiAgfQ0KICBtYWluKCk7DQp9KSgpOw0K";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("task_default"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
