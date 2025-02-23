use crate::artifacts::os::unix::cron::crontab::parse_cron;
use boa_engine::{js_string, Context, JsError, JsResult, JsValue};

/// Get `Cron` data
pub(crate) fn js_get_cron(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let cron = match parse_cron() {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to parse cron data: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let results = serde_json::to_value(&cron).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;
    Ok(value)
}

#[cfg(test)]
#[cfg(target_family = "unix")]
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

    #[test]
    fn test_get_cron() {
        let test = "Ly8gLi4vLi4vYXJ0ZW1pcy1hcGkvc3JjL3VuaXgvY3Jvbi50cwpmdW5jdGlvbiBnZXRfY3JvbigpIHsKICBjb25zdCBkYXRhID0ganNfZ2V0X2Nyb24oKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gLi4vLi4vYXJ0ZW1pcy1hcGkvbW9kLnRzCmZ1bmN0aW9uIGdldENyb24oKSB7CiAgcmV0dXJuIGdldF9jcm9uKCk7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBkYXRhID0gZ2V0Q3JvbigpOwogIHJldHVybiBkYXRhOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("cron_script"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
