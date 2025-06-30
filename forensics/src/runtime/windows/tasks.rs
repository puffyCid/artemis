use crate::{
    artifacts::os::windows::tasks::parser::grab_tasks, runtime::helper::string_arg,
    structs::artifacts::os::windows::TasksOptions,
};
use boa_engine::{Context, JsArgs, JsError, JsResult, JsValue, js_string};

/// Expose parsing Schedule Tasks to `BoaJS`
pub(crate) fn js_tasks(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = if args.get_or_undefined(0).is_undefined() {
        None
    } else {
        Some(string_arg(args, 0)?)
    };
    let options = TasksOptions { alt_file: path };
    let task = match grab_tasks(&options) {
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

    #[tokio::test]
    async fn test_js_tasks() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvd2luZG93cy90YXNrcy50cwpmdW5jdGlvbiBnZXRUYXNrcygpIHsKICBjb25zdCBkYXRhID0ganNfdGFza3MoKTsKICByZXR1cm4gZGF0YTsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IHRhc2tzID0gZ2V0VGFza3MoKTsKICBpZiAodGFza3MgaW5zdGFuY2VvZiBFcnJvcikgewogICAgY29uc29sZS5lcnJvcihgR290IHRhc2sgcGFyc2luZyBlcnJvciEgJHt0YXNrc31gKTsKICB9CiAgcmV0dXJuIHRhc2tzOwp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("task_default"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).await.unwrap();
    }
}
