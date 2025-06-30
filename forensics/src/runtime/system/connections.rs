use boa_engine::{Context, JsResult, JsValue};
use lumination::connections::connections;

pub(crate) fn js_connections(
    _this: &JsValue,
    _args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let conns = connections().unwrap_or_default();
    let results = serde_json::to_value(&conns).unwrap_or_default();
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
    async fn test_js_connections() {
        let test = "ZnVuY3Rpb24gbygpe3JldHVybiBqc19jb25uZWN0aW9ucygpfWZ1bmN0aW9uIG4oKXtyZXR1cm4gbygpfW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", true);
        let script = JSScript {
            name: String::from("connections"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).await.unwrap();
    }
}
