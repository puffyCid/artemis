use crate::utils::compression::decompress::{decompress_gzip_data, decompress_zlib};
use deno_core::{error::AnyError, op2, JsBuffer};

#[op2]
#[buffer]
/// Decompress zlib data
pub(crate) fn js_decompress_zlib(#[buffer] data: JsBuffer, wbits: u8) -> Result<Vec<u8>, AnyError> {
    let wbits_value = if wbits == 0 { None } else { Some(wbits) };
    let decom_data = decompress_zlib(&data, &wbits_value)?;
    Ok(decom_data)
}

#[op2]
#[buffer]
/// Expose decmpressing gzip data to Deno
pub(crate) fn js_decompress_gzip(#[buffer] data: JsBuffer) -> Result<Vec<u8>, AnyError> {
    let decom_data = decompress_gzip_data(&data)?;

    Ok(decom_data)
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
    fn test_zlib_decompress() {
        let test = "Ly8gLi4vLi4vUHJvamVjdHMvRGVuby9hcnRlbWlzLWFwaS9zcmMvdXRpbHMvZXJyb3IudHMKdmFyIEVycm9yQmFzZSA9IGNsYXNzIGV4dGVuZHMgRXJyb3IgewogIGNvbnN0cnVjdG9yKG5hbWUsIG1lc3NhZ2UpIHsKICAgIHN1cGVyKCk7CiAgICB0aGlzLm5hbWUgPSBuYW1lOwogICAgdGhpcy5tZXNzYWdlID0gbWVzc2FnZTsKICB9Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9EZW5vL2FydGVtaXMtYXBpL3NyYy9jb21wcmVzc2lvbi9lcnJvcnMudHMKdmFyIENvbXByZXNzaW9uRXJyb3IgPSBjbGFzcyBleHRlbmRzIEVycm9yQmFzZSB7Cn07CgovLyAuLi8uLi9Qcm9qZWN0cy9EZW5vL2FydGVtaXMtYXBpL3NyYy9jb21wcmVzc2lvbi9kZWNvbXByZXNzLnRzCmZ1bmN0aW9uIGRlY29tcHJlc3NfemxpYihkYXRhLCB3Yml0cyA9IDApIHsKICB0cnkgewogICAgY29uc3QgYnl0ZXMgPSBjb21wcmVzc2lvbi5kZWNvbXByZXNzX3psaWIoZGF0YSwgd2JpdHMpOwogICAgcmV0dXJuIGJ5dGVzOwogIH0gY2F0Y2ggKGVycikgewogICAgcmV0dXJuIG5ldyBDb21wcmVzc2lvbkVycm9yKGBaTElCYCwgYGZhaWxlZCB0byBkZWNvbXByZXNzOiAke2Vycn1gKTsKICB9Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCBkYXRhID0gbmV3IFVpbnQ4QXJyYXkoWwogICAgMTIwLAogICAgMTU2LAogICAgNSwKICAgIDEyOCwKICAgIDIwOSwKICAgIDksCiAgICAwLAogICAgMCwKICAgIDQsCiAgICA2OCwKICAgIDg3LAogICAgOTcsCiAgICA1NiwKICAgIDIyOSwKICAgIDIyNywKICAgIDE0OSwKICAgIDE5NCwKICAgIDIzNywKICAgIDEyNywKICAgIDExNywKICAgIDE5MywKICAgIDE5NiwKICAgIDIzNCwKICAgIDYyLAogICAgMTMsCiAgICAyNSwKICAgIDIxOCwKICAgIDQsCiAgICAzNgogIF0pOwogIGNvbnN0IGRlY29tX2RhdGEgPSBkZWNvbXByZXNzX3psaWIoZGF0YSk7CiAgY29uc29sZS5hc3NlcnQoZGVjb21fZGF0YS5sZW5ndGggPT09IDExKTsKfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("zlib_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_gzip_decompress() {
        let test = "";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("gzip_test"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
