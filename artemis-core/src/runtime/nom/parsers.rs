use deno_core::{error::AnyError, op, JsBuffer, ToJsBuffer};
use nom::bytes::complete::take;
use serde::Serialize;

#[derive(Serialize)]
struct NomStringJs {
    remaining: String,
    nommed: String,
}

#[op]
/// Expose nomming strings to Deno
fn js_nom_take_string(data: String, input: usize) -> Result<String, AnyError> {
    let (remaining, nommed) = nom_take_string(&data, input).unwrap_or_default();
    let nom_string = NomStringJs {
        remaining: remaining.to_string(),
        nommed,
    };
    let results = serde_json::to_string(&nom_string)?;

    Ok(results)
}

/// Expose `take` string function to Deno
fn nom_take_string(data: &str, input: usize) -> nom::IResult<&str, String> {
    let (remaining, nommed) = take(input)(data)?;
    Ok((remaining, nommed.to_string()))
}

#[derive(Serialize)]
struct NomBytesJs {
    remaining: ToJsBuffer,
    nommed: ToJsBuffer,
}

#[op]
/// Expose nomming bytes to Deno
fn js_nom_take_bytes(data: JsBuffer, input: usize) -> Result<NomBytesJs, AnyError> {
    let (remaining, nommed) = nom_take_bytes(&data, input).unwrap_or_default();
    let nom_bytes = NomBytesJs {
        remaining: remaining.to_vec().into(),
        nommed: nommed.into(),
    };

    Ok(nom_bytes)
}

/// Expose `take` bytes function to Deno
fn nom_take_bytes(data: &[u8], input: usize) -> nom::IResult<&[u8], Vec<u8>> {
    let (remaining, nommed) = take(input)(data)?;
    Ok((remaining, nommed.to_vec()))
}

#[cfg(test)]
mod tests {
    #[cfg(test)]
    mod tests {
        use crate::{
            runtime::{
                deno::execute_script,
                nom::parsers::{nom_take_bytes, nom_take_string},
            },
            structs::artifacts::runtime::script::JSScript,
            utils::artemis_toml::Output,
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
        fn test_js_nom_take_string() {
            let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2Rpbmcvc3RyaW5ncy50cwpmdW5jdGlvbiBleHRyYWN0VXRmOFN0cmluZyhkYXRhKSB7CiAgY29uc3QgcmVzdWx0ID0gZW5jb2RpbmcuZXh0cmFjdF91dGY4X3N0cmluZyhkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9lbmNvZGluZy9ieXRlcy50cwpmdW5jdGlvbiBlbmNvZGVCeXRlcyhkYXRhKSB7CiAgY29uc3QgcmVzdWx0ID0gZW5jb2RpbmcuYnl0ZXNfZW5jb2RlKGRhdGEpOwogIHJldHVybiByZXN1bHQ7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL25vbS9wYXJzZXJzLnRzCmZ1bmN0aW9uIHRha2UoZGF0YSwgaW5wdXQpIHsKICBpZiAoaW5wdXQgPCAwKSB7CiAgICBjb25zdCBlcnIgPSBuZXcgRXJyb3IoInByb3ZpZGVkIG5lZ2F0aXZlIG51bWJlciIpOwogICAgcmV0dXJuIGVycjsKICB9CiAgaWYgKHR5cGVvZiBkYXRhID09PSAic3RyaW5nIikgewogICAgY29uc3QgcmVzdWx0X3N0cmluZyA9IERlbm8uY29yZS5vcHMuanNfbm9tX3Rha2Vfc3RyaW5nKGRhdGEsIGlucHV0KTsKICAgIGNvbnN0IG5vbV9zdHJpbmcgPSBKU09OLnBhcnNlKHJlc3VsdF9zdHJpbmcpOwogICAgcmV0dXJuIG5vbV9zdHJpbmc7CiAgfQogIGNvbnN0IHJlc3VsdCA9IERlbm8uY29yZS5vcHMuanNfbm9tX3Rha2VfYnl0ZXMoZGF0YSwgaW5wdXQpOwogIGNvbnN0IG5vbSA9IHJlc3VsdDsKICByZXR1cm4gbm9tOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgcmVzdWx0ID0gdGFrZSgiaGVsbG8gd29ybGQhIiwgNSk7CiAgaWYgKHJlc3VsdCBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICBjb25zb2xlLmVycm9yKGBGYWlsZWQgdG8gbm9tIHN0cmluZzogJHtyZXN1bHR9YCk7CiAgICByZXR1cm4gcmVzdWx0OwogIH0KICBjb25zb2xlLmxvZygKICAgIGBJIG5vbW1lZDogJyR7cmVzdWx0Lm5vbW1lZH0nLiBJIGhhdmUgcmVtYWluaW5nOiAnJHtyZXN1bHQucmVtYWluaW5nfSdgLAogICk7CiAgY29uc3QgYnl0ZXMgPSBlbmNvZGVCeXRlcygiaGVsbG8gd29ybGQhIik7CiAgY29uc3QgcnN1bHRCeXRlcyA9IHRha2UoYnl0ZXMsIDUpOwogIGlmIChyc3VsdEJ5dGVzIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYEZhaWxlZCB0byBub20gYnl0ZXM6ICR7cmVzdWx0fWApOwogICAgcmV0dXJuIHJlc3VsdDsKICB9CiAgY29uc29sZS5sb2coCiAgICBgSSBub21tZWQgYnl0ZXM6ICR7cnN1bHRCeXRlcy5ub21tZWR9LiBTdHJpbmcgcmVtYWluaW5nICh2aWEgYnl0ZXMgdG8gc3RyaW5nKTogJHsKICAgICAgZXh0cmFjdFV0ZjhTdHJpbmcocnN1bHRCeXRlcy5yZW1haW5pbmcpCiAgICB9YCwKICApOwp9Cm1haW4oKTsK";
            let mut output = output_options("runtime_test", "local", "./tmp", false);
            let script = JSScript {
                name: String::from("nomming"),
                script: test.to_string(),
            };
            execute_script(&mut output, &script).unwrap();
        }

        #[test]
        fn test_js_nom_take_bytes() {
            let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvZW5jb2Rpbmcvc3RyaW5ncy50cwpmdW5jdGlvbiBleHRyYWN0VXRmOFN0cmluZyhkYXRhKSB7CiAgY29uc3QgcmVzdWx0ID0gZW5jb2RpbmcuZXh0cmFjdF91dGY4X3N0cmluZyhkYXRhKTsKICByZXR1cm4gcmVzdWx0Owp9CgovLyBodHRwczovL3Jhdy5naXRodWJ1c2VyY29udGVudC5jb20vcHVmZnljaWQvYXJ0ZW1pcy1hcGkvbWFzdGVyL3NyYy9lbmNvZGluZy9ieXRlcy50cwpmdW5jdGlvbiBlbmNvZGVCeXRlcyhkYXRhKSB7CiAgY29uc3QgcmVzdWx0ID0gZW5jb2RpbmcuYnl0ZXNfZW5jb2RlKGRhdGEpOwogIHJldHVybiByZXN1bHQ7Cn0KCi8vIGh0dHBzOi8vcmF3LmdpdGh1YnVzZXJjb250ZW50LmNvbS9wdWZmeWNpZC9hcnRlbWlzLWFwaS9tYXN0ZXIvc3JjL25vbS9wYXJzZXJzLnRzCmZ1bmN0aW9uIHRha2UoZGF0YSwgaW5wdXQpIHsKICBpZiAoaW5wdXQgPCAwKSB7CiAgICBjb25zdCBlcnIgPSBuZXcgRXJyb3IoInByb3ZpZGVkIG5lZ2F0aXZlIG51bWJlciIpOwogICAgcmV0dXJuIGVycjsKICB9CiAgaWYgKHR5cGVvZiBkYXRhID09PSAic3RyaW5nIikgewogICAgY29uc3QgcmVzdWx0X3N0cmluZyA9IERlbm8uY29yZS5vcHMuanNfbm9tX3Rha2Vfc3RyaW5nKGRhdGEsIGlucHV0KTsKICAgIGNvbnN0IG5vbV9zdHJpbmcgPSBKU09OLnBhcnNlKHJlc3VsdF9zdHJpbmcpOwogICAgcmV0dXJuIG5vbV9zdHJpbmc7CiAgfQogIGNvbnN0IHJlc3VsdCA9IERlbm8uY29yZS5vcHMuanNfbm9tX3Rha2VfYnl0ZXMoZGF0YSwgaW5wdXQpOwogIGNvbnN0IG5vbSA9IHJlc3VsdDsKICByZXR1cm4gbm9tOwp9CgovLyBtYWluLnRzCmZ1bmN0aW9uIG1haW4oKSB7CiAgY29uc3QgcmVzdWx0ID0gdGFrZSgiaGVsbG8gd29ybGQhIiwgNSk7CiAgaWYgKHJlc3VsdCBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICBjb25zb2xlLmVycm9yKGBGYWlsZWQgdG8gbm9tIHN0cmluZzogJHtyZXN1bHR9YCk7CiAgICByZXR1cm4gcmVzdWx0OwogIH0KICBjb25zb2xlLmxvZygKICAgIGBJIG5vbW1lZDogJyR7cmVzdWx0Lm5vbW1lZH0nLiBJIGhhdmUgcmVtYWluaW5nOiAnJHtyZXN1bHQucmVtYWluaW5nfSdgLAogICk7CiAgY29uc3QgYnl0ZXMgPSBlbmNvZGVCeXRlcygiaGVsbG8gd29ybGQhIik7CiAgY29uc3QgcnN1bHRCeXRlcyA9IHRha2UoYnl0ZXMsIDUpOwogIGlmIChyc3VsdEJ5dGVzIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYEZhaWxlZCB0byBub20gYnl0ZXM6ICR7cmVzdWx0fWApOwogICAgcmV0dXJuIHJlc3VsdDsKICB9CiAgY29uc29sZS5sb2coCiAgICBgSSBub21tZWQgYnl0ZXM6ICR7cnN1bHRCeXRlcy5ub21tZWR9LiBTdHJpbmcgcmVtYWluaW5nICh2aWEgYnl0ZXMgdG8gc3RyaW5nKTogJHsKICAgICAgZXh0cmFjdFV0ZjhTdHJpbmcocnN1bHRCeXRlcy5yZW1haW5pbmcpCiAgICB9YCwKICApOwp9Cm1haW4oKTsK";
            let mut output = output_options("runtime_test", "local", "./tmp", false);
            let script = JSScript {
                name: String::from("nomming"),
                script: test.to_string(),
            };
            execute_script(&mut output, &script).unwrap();
        }

        #[test]
        fn test_nom_take_string() {
            let test = "hello world!!";
            nom_take_string(&test, test.len() - 3).unwrap();
        }

        #[test]
        fn test_nom_take_bytes() {
            let test = b"hello world!!";
            nom_take_bytes(test, 4).unwrap();
        }
    }
}
