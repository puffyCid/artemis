use crate::runtime::helper::{number_arg, string_arg};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};
use log::warn;
use serde_json::{Value, json};

pub(crate) fn js_read_csv(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let path = string_arg(args, 0)?;
    let offset = number_arg(args, 1)?;
    let limit = number_arg(args, 2)?;

    let mut reader = match csv::Reader::from_path(&path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not create reader for {path}: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
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
        if count > limit as u64 {
            break;
        }
        if value.len() != headers.len() {
            warn!(
                "[forensics] Headers and csv row do not match. Headers: {}. Row: {}",
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
