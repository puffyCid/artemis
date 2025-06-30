use crate::{
    runtime::helper::string_arg,
    utils::environment::{get_env, get_env_value},
};
use boa_engine::{Context, JsResult, JsValue};

/// Get all Environmental variables
pub(crate) fn js_env(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let env = get_env();
    let serde_value = serde_json::to_value(&env).unwrap_or_default();

    let value = JsValue::from_json(&serde_value, context)?;
    Ok(value)
}

/// Get a specific Environmental variable
pub(crate) fn js_env_value(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let input = string_arg(args, 0)?;

    let env = get_env_value(&input);
    let serde_value = serde_json::to_value(env).unwrap_or_default();
    let value = JsValue::from_json(&serde_value, context)?;
    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::runtime::run::execute_script;
    use crate::structs::artifacts::runtime::script::JSScript;
    use crate::structs::toml::Output;

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
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[tokio::test]
    async fn test_js_env() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW52aXJvbm1lbnQvZW52LnRzCmZ1bmN0aW9uIGxpc3RFbnYoKSB7CiAgY29uc3QgZGF0YSA9IGpzX2VudigpOwogIHJldHVybiBkYXRhOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZW52cyA9IGxpc3RFbnYoKTsKICByZXR1cm4gZW52czsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("envs_list"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).await.unwrap();
    }

    #[tokio::test]
    #[cfg(target_family = "unix")]
    async fn test_js_env_value() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW52aXJvbm1lbnQvZW52LnRzCmZ1bmN0aW9uIGdldEVudlZhbHVlKGtleSkgewogIGNvbnN0IGRhdGEgPSBqc19lbnZfdmFsdWUoa2V5KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGVudnMgPSBnZXRFbnZWYWx1ZSgiUFdEIik7CiAgcmV0dXJuIGVudnM7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("env_pwd"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).await.unwrap();
    }

    #[tokio::test]
    #[cfg(target_os = "windows")]
    async fn test_js_env_value() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW52aXJvbm1lbnQvZW52LnRzCmZ1bmN0aW9uIGdldEVudlZhbHVlKGtleSkgewogIGNvbnN0IGRhdGEgPSBqc19lbnZfdmFsdWUoa2V5KTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGVudnMgPSBnZXRFbnZWYWx1ZSgiU3lzdGVtRHJpdmUiKTsKICByZXR1cm4gZW52czsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("env_pwd"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).await.unwrap();
    }
}
