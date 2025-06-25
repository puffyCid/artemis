use crate::{
    artifacts::os::macos::spotlight::light::{
        StoreMeta, parse_spotlight_reader, setup_spotlight_reader,
    },
    runtime::helper::{bigint_arg, string_arg, value_arg},
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string};

/// Expose parsing Spotlight to `BoaJS`
pub(crate) fn js_spotlight(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let store_file = string_arg(args, &0)?;
    let meta = value_arg(args, &1, context)?;
    let offset = bigint_arg(args, &2)? as u32;

    let serde_result = serde_json::from_value(meta);
    let store_meta: StoreMeta = match serde_result {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to deserialize store metadata: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let entries =
        match parse_spotlight_reader(&store_file, &store_meta.meta, &store_meta.blocks, offset) {
            Ok(result) => result,
            Err(err) => {
                let issue = format!("Failed to get spotlight: {err:?}");
                return Err(JsError::from_opaque(js_string!(issue).into()));
            }
        };

    let results = serde_json::to_value(&entries).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Expose setting up Spotlight parser to `BoaJS`
pub(crate) fn js_setup_spotlight_parser(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let glob_path = string_arg(args, &0)?;

    let meta = match setup_spotlight_reader(&glob_path) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to setup spotlight parser: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };

    let results = serde_json::to_value(&meta).unwrap_or_default();
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
            filter_name: Some(String::new()),
            filter_script: Some(String::new()),
            logging: Some(String::new()),
        }
    }

    #[test]
    fn test_js_spotlight() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21haW4vc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21haW4vc3JjL2ZpbGVzeXN0ZW0vZXJyb3JzLnRzCnZhciBGaWxlRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9EZW5vL2FydGVtaXMtYXBpL3NyYy91dGlscy9lcnJvci50cwp2YXIgRXJyb3JCYXNlMiA9IGNsYXNzIGV4dGVuZHMgRXJyb3IgewogIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgIHN1cGVyKCk7CiAgICB0aGlzLm5hbWUgPSBuYW1lOwogICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICB9Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9EZW5vL2FydGVtaXMtYXBpL3NyYy9tYWNvcy9lcnJvcnMudHMKdmFyIE1hY29zRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZTIgewp9OwoKLy8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvbWFjb3Mvc3BvdGxpZ2h0LnRzCmZ1bmN0aW9uIHNldHVwX3Nwb3RsaWdodF9wYXJzZXIoZ2xvYl9wYXRoKSB7CiAgdHJ5IHsKICAgIGNvbnN0IGRhdGEgPSBqc19zZXR1cF9zcG90bGlnaHRfcGFyc2VyKGdsb2JfcGF0aCk7CiAgICByZXR1cm4gZGF0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgTWFjb3NFcnJvcigKICAgICAgIlNQT1RMSUdIVCIsCiAgICAgIGBmYWlsZWQgdG8gc2V0dXAgc3BvdGxpZ2h0IHBhcnNlciBmb3IgJHtnbG9iX3BhdGh9OiAke2Vycn1gCiAgICApOwogIH0KfQpmdW5jdGlvbiBnZXRfc3BvdGxpZ2h0KG1ldGEsIHN0b3JlX2ZpbGUsIG9mZnNldCkgewogIHRyeSB7CiAgICBjb25zdCBkYXRhID0ganNfc3BvdGxpZ2h0KHN0b3JlX2ZpbGUsIG1ldGEsIG9mZnNldCk7CiAgICByZXR1cm4gZGF0YTsKICB9IGNhdGNoIChlcnIpIHsKICAgIHJldHVybiBuZXcgTWFjb3NFcnJvcigKICAgICAgIlNQT1RMSUdIVCIsCiAgICAgIGBmYWlsZWQgdG8gZ2V0IHNwb3RsaWdodCBlbnRyaWVzIGZvciAke3N0b3JlX2ZpbGV9OiAke2Vycn1gCiAgICApOwogIH0KfQoKLy8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21haW4vc3JjL2ZpbGVzeXN0ZW0vZmlsZXMudHMKZnVuY3Rpb24gZ2xvYihwYXR0ZXJuKSB7CiAgdHJ5IHsKICAgIGNvbnN0IHJlc3VsdCA9IGpzX2dsb2IocGF0dGVybik7CiAgICByZXR1cm4gcmVzdWx0OwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBGaWxlRXJyb3IoIkdMT0IiLCBgZmFpbGVkIHRvIGdsb2IgcGF0dGVybiAke3BhdHRlcm59IiAke2Vycn1gKTsKICB9Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBwYXRoID0gIi9Vc2Vycy8qL0xpYnJhcnkvQ2FjaGVzL2NvbS5hcHBsZS5oZWxwZC9pbmRleC5zcG90bGlnaHRWMy8qIjsKICBjb25zdCBtZXRhID0gc2V0dXBfc3BvdGxpZ2h0X3BhcnNlcihwYXRoKTsKICBpZiAobWV0YSBpbnN0YW5jZW9mIE1hY29zRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYENvdWxkIG5vdCBzZXR1cCBzcG90bGlnaHQgcGFyc2VyOiAke21ldGF9YCk7CiAgICByZXR1cm4gW107CiAgfQogIGNvbnN0IHN0b3JlX3BhdGggPSBnbG9iKCIvVXNlcnMvKi9MaWJyYXJ5L0NhY2hlcy9jb20uYXBwbGUuaGVscGQvaW5kZXguc3BvdGxpZ2h0VjMvc3RvcmUuZGIiKTsKICBpZiAoc3RvcmVfcGF0aCBpbnN0YW5jZW9mIEZpbGVFcnJvcikgewogICAgY29uc29sZS5lcnJvcihgQ291bGQgZ2xvYiBzcG90bGlnaHQgc3RvcmUuZGI6ICR7c3RvcmVfcGF0aH1gKTsKICAgIHJldHVybiBbXTsKICB9CiAgZm9yIChjb25zdCBwYXRoMiBvZiBzdG9yZV9wYXRoKSB7CiAgICBjb25zdCByZXN1bHRzID0gZ2V0X3Nwb3RsaWdodChtZXRhLCBwYXRoMi5mdWxsX3BhdGgsIDApOwogICAgaWYgKHJlc3VsdHMgaW5zdGFuY2VvZiBNYWNvc0Vycm9yKSB7CiAgICAgIGNvbnNvbGUuZXJyb3IoYENvdWxkIG5vdCBwYXJzZSBzcG90bGlnaHQgZGIgJHtwYXRoMi5mdWxsX3BhdGh9OiAke3Jlc3VsdHN9YCk7CiAgICAgIHJldHVybiBbXTsKICAgIH0KICAgIGNvbnNvbGUubG9nKHJlc3VsdHNbMF0pOwogICAgcmV0dXJuIHJlc3VsdHM7CiAgfQp9Cm1haW4oKTsK";
        let mut output = output_options("runtime_test", "local", "./tmp", false);

        let script = JSScript {
            name: String::from("spotlight_script"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
