use crate::utils::encoding::parse_protobuf;
use deno_core::{error::AnyError, op2, JsBuffer};

#[op2]
#[string]
/// Parse Protobuf data
pub(crate) fn js_parse_protobuf(#[buffer] data: JsBuffer) -> Result<String, AnyError> {
    let result = parse_protobuf(&data)?;

    let res = serde_json::to_string(&result)?;

    Ok(res)
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::deno::execute_script, structs::artifacts::runtime::script::JSScript,
        structs::toml::Output,
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
        let test = "";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("protobuf_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
