use crate::{
    runtime::helper::{bytes_arg, number_arg},
    utils::compression::decompress::{decompress_gzip_data, decompress_zlib},
};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string, object::builtins::JsUint8Array};

/// Decompress zlib data
pub(crate) fn js_decompress_zlib(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let data = bytes_arg(args, 0, context)?;
    let wbits = number_arg(args, 1)? as i32;
    let decom_size = number_arg(args, 2)? as usize;

    let wbits_value = if wbits == 0 { None } else { Some(wbits) };
    let decom_data = match decompress_zlib(&data, wbits_value, decom_size) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not get decompress data: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let bytes = JsUint8Array::from_iter(decom_data, context)?;

    Ok(bytes.into())
}

/// Expose decmpressing gzip data to Boa
pub(crate) fn js_decompress_gzip(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let data = bytes_arg(args, 0, context)?;

    let decom_data = match decompress_gzip_data(&data) {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Could not get decompress data: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let bytes = JsUint8Array::from_iter(decom_data, context)?;

    Ok(bytes.into())
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

    #[test]
    fn test_zlib_decompress() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvdXRpbHMvZXJyb3IudHMKdmFyIEVycm9yQmFzZSA9IGNsYXNzIGV4dGVuZHMgRXJyb3IgewogIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgIHN1cGVyKCk7CiAgICB0aGlzLm5hbWUgPSBuYW1lOwogICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICB9Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9EZW5vL2FydGVtaXMtYXBpL3NyYy9jb21wcmVzc2lvbi9lcnJvcnMudHMKdmFyIENvbXByZXNzaW9uRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9EZW5vL2FydGVtaXMtYXBpL3NyYy9jb21wcmVzc2lvbi9kZWNvbXByZXNzLnRzCmZ1bmN0aW9uIGRlY29tcHJlc3NfemxpYihkYXRhLCB3Yml0cyA9IDApIHsKICB0cnkgewogICAgY29uc3QgYnl0ZXMgPSBqc19kZWNvbXByZXNzX3psaWIoZGF0YSwgd2JpdHMsIDApOwogICAgcmV0dXJuIGJ5dGVzOwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBDb21wcmVzc2lvbkVycm9yKGBaTElCYCwgYGZhaWxlZCB0byBkZWNvbXByZXNzOiAke2Vycn1gKTsKICB9Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBkYXRhID0gbmV3IFVpbnQ4QXJyYXkoWwogICAgMTIwLAogICAgMTU2LAogICAgNSwKICAgIDEyOCwKICAgIDIwOSwKICAgIDksCiAgICAwLAogICAgMCwKICAgIDQsCiAgICA2OCwKICAgIDg3LAogICAgOTcsCiAgICA1NiwKICAgIDIyOSwKICAgIDIyNywKICAgIDE0OSwKICAgIDE5NCwKICAgIDIzNywKICAgIDEyNywKICAgIDExNywKICAgIDE5MywKICAgIDE5NiwKICAgIDIzNCwKICAgIDYyLAogICAgMTMsCiAgICAyNSwKICAgIDIxOCwKICAgIDQsCiAgICAzNgogIF0pOwogIGNvbnN0IGRlY29tX2RhdGEgPSBkZWNvbXByZXNzX3psaWIoZGF0YSk7CiAgY29uc29sZS5hc3NlcnQoZGVjb21fZGF0YS5sZW5ndGggPT09IDExKTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("zlib_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_gzip_decompress() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL3V0aWxzL2Vycm9yLnRzCnZhciBFcnJvckJhc2UgPSBjbGFzcyBleHRlbmRzIEVycm9yIHsKICBjb25zdHJ1Y3RvcihuYW1lLCBtZXNzYWdlKSB7CiAgICBzdXBlcigpOwogICAgdGhpcy5uYW1lID0gbmFtZTsKICAgIHRoaXMubWVzc2FnZSA9IG1lc3NhZ2U7CiAgfQp9OwoKLy8gLi4vLi4vUHJvamVjdHMvYXJ0ZW1pcy1hcGkvc3JjL2NvbXByZXNzaW9uL2Vycm9ycy50cwp2YXIgQ29tcHJlc3Npb25FcnJvciA9IGNsYXNzIGV4dGVuZHMgRXJyb3JCYXNlIHsKfTsKCi8vIC4uLy4uL1Byb2plY3RzL2FydGVtaXMtYXBpL3NyYy9jb21wcmVzc2lvbi9kZWNvbXByZXNzLnRzCmZ1bmN0aW9uIGRlY29tcHJlc3NfZ3ppcChkYXRhKSB7CiAgdHJ5IHsKICAgIGNvbnN0IGJ5dGVzID0ganNfZGVjb21wcmVzc19nemlwKGRhdGEpOwogICAgcmV0dXJuIGJ5dGVzOwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBDb21wcmVzc2lvbkVycm9yKGBHWklQYCwgYGZhaWxlZCB0byBkZWNvbXByZXNzOiAke2Vycn1gKTsKICB9Cn0KCi8vIC4uLy4uL1Byb2plY3RzL2FydGVtaXMtYXBpL3NyYy9lbmNvZGluZy9lcnJvcnMudHMKdmFyIEVuY29kaW5nRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvZW5jb2RpbmcvYmFzZTY0LnRzCmZ1bmN0aW9uIGRlY29kZShiNjQpIHsKICB0cnkgewogICAgY29uc3QgYnl0ZXMgPSBqc19iYXNlNjRfZGVjb2RlKGI2NCk7CiAgICByZXR1cm4gYnl0ZXM7CiAgfSBjYXRjaCAoZXJyKSB7CiAgICByZXR1cm4gbmV3IEVuY29kaW5nRXJyb3IoYEJBU0U2NGAsIGBmYWlsZWQgdG8gZGVjb2RlICR7YjY0fTogJHtlcnJ9YCk7CiAgfQp9CgovLyAuLi8uLi9Qcm9qZWN0cy9hcnRlbWlzLWFwaS9zcmMvZW5jb2Rpbmcvc3RyaW5ncy50cwpmdW5jdGlvbiBleHRyYWN0VXRmOFN0cmluZyhkYXRhKSB7CiAgY29uc3QgcmVzdWx0ID0ganNfZXh0cmFjdF91dGY4X3N0cmluZyhkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgZGF0YSA9IGRlY29kZSgiSDRzSUFIaFhqMllBL3dXQVFRa0FBQWdEcTJnM0JSOEhnMzJzUDI1QjlUTFRBVzNDdEFNTUFBQUEiKTsKICBjb25zdCB2YWx1ZSA9IGRlY29tcHJlc3NfZ3ppcChkYXRhKTsKICBjb25zdCB0ZXh0ID0gZXh0cmFjdFV0ZjhTdHJpbmcodmFsdWUpOwogIGlmICh0ZXh0ICE9ICJoZWxsbyB3b3JsZCEiKSB7CiAgICB0aHJvdyAiYmFkIGRlY29tcHJlc3NzaW9uISI7CiAgfQogIGNvbnNvbGUubG9nKGBJIGRlY29tcHJlc3NlZCAke3RleHR9YCk7Cn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("gzip_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
