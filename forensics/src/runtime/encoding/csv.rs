use crate::{
    accessor::access::Accessor,
    runtime::{
        filesystem::accessor::return_file_handle_object,
        helper::{number_arg, value_arg},
    },
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};
use serde_json::{Value, json};
use tracing::warn;

/// Parse a CSV file into array of JSON objects
pub(crate) fn js_read_csv(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let value = value_arg(args, 0, context)?;
    let offset = number_arg(args, 1)?;
    let limit = number_arg(args, 2)?;

    let handle = return_file_handle_object(value)?;
    let path = handle.display_path();
    let mut reader = match Accessor::with_defaults().open_reader_handle(&handle) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not create reader for {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let mut reader = csv::Reader::from_reader(&mut reader);
    let headers = match reader.headers().cloned() {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not get headers for {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let mut iter = reader.records().skip(offset as usize);
    let mut csv_value = Vec::new();
    let mut count = 0;
    while let Some(Ok(value)) = iter.next() {
        if count == limit as u64 {
            break;
        }
        if value.len() != headers.len() {
            warn!(
                "Headers and csv row do not match. Headers: {}. Row: {}",
                headers.len(),
                value.len()
            );
            continue;
        }

        let mut index = 0;
        let mut entry = json!({});
        while index < headers.len() {
            entry[headers[index].to_string()] = Value::String(value[index].to_string());
            index += 1;
        }
        csv_value.push(entry);
        count += 1;
    }

    let json_array = Value::Array(csv_value);

    let value = JsValue::from_json(&json_array, context)?;

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
    fn test_js_read_csv() {
        let mut test_location = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        test_location.push("tests/test_data/runtime/encoding/csv.js");

        let bytes = fs::read(test_location).unwrap();
        let test = base64_encode_standard(&bytes);

        let mut output = output_options("runtime_test", "./tmp", false);
        let script = JSScript {
            name: String::from("csv_test_failed"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
