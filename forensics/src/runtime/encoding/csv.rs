use crate::runtime::helper::{number_arg, string_arg};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};
use log::warn;
use serde_json::{Value, json};

/// Parse a CSV file into array of JSON objects
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

#[cfg(test)]
mod tests {
    use crate::runtime::run::execute_script;
    use crate::{structs::artifacts::runtime::script::JSScript, structs::toml::Output};

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

    #[test]
    fn test_js_read_csv() {
        let test = "KCgpID0+IHsKICAvLyAuLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvdXRpbHMvZXJyb3IudHMKICB2YXIgRXJyb3JCYXNlID0gY2xhc3MgZXh0ZW5kcyBFcnJvciB7CiAgICBuYW1lOwogICAgbWVzc2FnZTsKICAgIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgICAgc3VwZXIoKTsKICAgICAgdGhpcy5uYW1lID0gbmFtZTsKICAgICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICAgIH0KICB9OwoKICAvLyAuLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvZW5jb2RpbmcvZXJyb3JzLnRzCiAgdmFyIEVuY29kaW5nRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7CiAgfTsKCiAgLy8gLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL2VuY29kaW5nL2Nzdi50cwogIGZ1bmN0aW9uIHJlYWRDc3YocGF0aCwgb2Zmc2V0ID0gMCwgbGltaXQgPSAxMDApIHsKICAgIHRyeSB7CiAgICAgIGNvbnN0IHJlc3VsdCA9IGpzX3JlYWRfY3N2KHBhdGgsIG9mZnNldCwgbGltaXQpOwogICAgICByZXR1cm4gcmVzdWx0OwogICAgfSBjYXRjaCAoZXJyKSB7CiAgICAgIHJldHVybiBuZXcgRW5jb2RpbmdFcnJvcigiQ1NWIiwgYGZhaWxlZCB0byByZWFkIENTViAke3BhdGh9OiAke2Vycn1gKTsKICAgIH0KICB9CgogIC8vIG1haW4udHMKICBmdW5jdGlvbiBtYWluKCkgewogICAgY29uc3QgZGF0YSA9IHJlYWRDc3YoInRlc3QuY3N2Iik7CiAgICBjb25zb2xlLmxvZyhKU09OLnN0cmluZ2lmeShkYXRhKSk7CiAgfQogIG1haW4oKTsKfSkoKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("csv_test_failed"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
