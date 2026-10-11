use crate::{
    artifacts::os::windows::eventlogs::parser::parse_eventlogs,
    runtime::{
        filesystem::accessor::return_file_handle_object,
        helper::{boolean_arg, number_arg, string_arg, value_arg},
    },
};
use boa_engine::{Context, JsArgs, JsError, JsResult, JsValue, js_string};

/// Expose parsing a single eventlog file (evtx) to `BoaJS`
pub(crate) fn js_eventlogs(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let file_value = value_arg(args, 0, context)?;
    let offset = number_arg(args, 1)? as usize;
    let limit = number_arg(args, 2)? as usize;
    let include_templates = boolean_arg(args, 3)?;

    let handle = return_file_handle_object(file_value)?;

    let temp_option = if args.get_or_undefined(4).is_undefined() {
        None
    } else {
        Some(string_arg(args, 4)?)
    };

    let logs = match parse_eventlogs(
        &handle,
        offset as usize,
        limit as usize,
        include_templates,
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
    use crate::structs::toml::{OutputConfig, OutputDestination, OutputFormat};
    use crate::utils::encoding::base64_encode_standard;
    use crate::{
        output::manager::OutputManager, runtime::run::execute_script,
        structs::artifacts::runtime::script::JSScript,
    };
    use std::fs;
    use std::path::PathBuf;

    fn output_options(name: &str, directory: &str, compress: bool) -> OutputManager {
        let config = OutputConfig {
            name: name.to_string(),
            directory: PathBuf::from(directory),
            format: OutputFormat::Jsonl,
            compress,
            endpoint_id: String::from("abcd"),
            destination: OutputDestination::Local,
            ..Default::default()
        };
        OutputManager::new(config).unwrap()
    }

    #[test]
    fn test_js_eventlogs() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/runtime/eventlogs/main.js");

        let bytes = fs::read(test_location).unwrap();
        let test = base64_encode_standard(&bytes);

        let mut output = output_options("runtime_test", "./tmp", false);
        let script = JSScript {
            name: String::from("js_eventlogs"),
            script: test,
        };
        execute_script(&mut output, &script).unwrap();
    }
}
