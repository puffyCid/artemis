use crate::{runtimev2::helper::bytes_arg, utils::encoding::parse_protobuf};
use boa_engine::{js_string, Context, JsError, JsResult, JsValue};

/// Parse Protobuf data
pub(crate) fn js_parse_protobuf(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let data = bytes_arg(args, &0, context)?;

    let result = match parse_protobuf(&data) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not get decode protobuf: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let res = serde_json::to_value(&result).unwrap_or_default();
    let value = JsValue::from_json(&res, context)?;

    Ok(value)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtimev2::run::execute_script,
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
    fn test_js_parse_protobuf() {
        let test = "TODO";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("protobuf_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
