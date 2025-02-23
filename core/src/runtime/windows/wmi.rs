use crate::{
    artifacts::os::windows::wmi::parser::grab_wmi_persist, runtime::helper::string_arg,
    structs::artifacts::os::windows::WmiPersistOptions,
};
use boa_engine::{js_string, Context, JsArgs, JsError, JsResult, JsValue};

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
    fn test_js_wmipersist() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3MvZXJyb3JzLnRzCnZhciBXaW5kb3dzRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvZW52aXJvbm1lbnQvZW52LnRzCmZ1bmN0aW9uIGdldEVudlZhbHVlKGtleSkgewogIGNvbnN0IGRhdGEgPSBqc19lbnZfdmFsdWUoa2V5KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3dpbmRvd3Mvd21pLnRzCmZ1bmN0aW9uIGdldFdtaVBlcnNpc3QoKSB7CiAgdHJ5IHsKICAgIGNvbnN0IGRhdGEgPSBqc193bWlwZXJzaXN0KCk7CiAgICByZXR1cm4gZGF0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgV2luZG93c0Vycm9yKCJXTUlQRVJTSVNUIiwgYGZhaWxlZCB0byBwYXJzZSBXTUkgcmVwbzogJHtlcnJ9YCk7CiAgfQp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZGF0YSA9IGdldFdtaVBlcnNpc3QoKTsKICByZXR1cm4gZGF0YTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("wmipersist"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
