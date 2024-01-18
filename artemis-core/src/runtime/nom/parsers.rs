use deno_core::{anyhow::anyhow, error::AnyError, op2, JsBuffer, ToJsBuffer};
use nom::bytes::complete::{take, take_until, take_while};
use serde::Serialize;

#[derive(Serialize)]
struct NomStringJs {
    remaining: String,
    nommed: String,
}

#[op2]
#[string]
/// Expose nomming strings to Deno
pub(crate) fn js_nom_take_string(
    #[string] data: String,
    #[bigint] input: usize,
) -> Result<String, AnyError> {
    let results = nom_take_string(&data, input);
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(_) => return Err(anyhow!("Failed to nom string")),
    };
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
pub(crate) struct NomBytesJs {
    remaining: ToJsBuffer,
    nommed: ToJsBuffer,
}

#[op2]
#[serde]
/// Expose nomming bytes to Deno
pub(crate) fn js_nom_take_bytes(
    #[buffer] data: JsBuffer,
    #[bigint] input: usize,
) -> Result<NomBytesJs, AnyError> {
    let results = nom_take_bytes(&data, input);
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(_) => return Err(anyhow!("Failed to nom bytes")),
    };
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

#[op2]
#[string]
/// Expose `take_until` string function to Deno
pub(crate) fn js_nom_take_until_string(
    #[string] data: String,
    #[string] input: String,
) -> Result<String, AnyError> {
    let results = nom_take_until_string(&data, &input);
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(_) => return Err(anyhow!("Failed to nom until string")),
    };
    let nom_string = NomStringJs {
        remaining: remaining.to_string(),
        nommed,
    };
    let results = serde_json::to_string(&nom_string)?;

    Ok(results)
}

/// Expose `take_until` string function to Deno
fn nom_take_until_string<'a>(data: &'a str, input: &str) -> nom::IResult<&'a str, String> {
    let (remaining, nommed) = take_until(input)(data)?;
    Ok((remaining, nommed.to_string()))
}

#[op2]
#[serde]
/// Expose `take_until` bytes function to Deno
pub(crate) fn js_nom_take_until_bytes(
    #[buffer] data: JsBuffer,
    #[buffer] input: JsBuffer,
) -> Result<NomBytesJs, AnyError> {
    let results = nom_take_until_bytes(&data, &input);
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(_) => return Err(anyhow!("Failed to nom until bytes")),
    };
    let nom_bytes = NomBytesJs {
        remaining: remaining.to_vec().into(),
        nommed: nommed.into(),
    };

    Ok(nom_bytes)
}

/// Expose `take_until` bytes function to Deno
fn nom_take_until_bytes<'a>(data: &'a [u8], input: &[u8]) -> nom::IResult<&'a [u8], Vec<u8>> {
    let (remaining, nommed) = take_until(input)(data)?;
    Ok((remaining, nommed.to_vec()))
}

#[op2]
#[string]
/// Expose `take_while` string function to Deno
pub(crate) fn js_nom_take_while_string(
    #[string] data: String,
    #[serde] input: char,
) -> Result<String, AnyError> {
    let results = nom_take_while_string(&data, input);
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(_) => return Err(anyhow!("Failed to nom while string")),
    };
    let nom_string = NomStringJs {
        remaining: remaining.to_string(),
        nommed,
    };
    let results = serde_json::to_string(&nom_string)?;

    Ok(results)
}

/// Expose `take_while` string function to Deno
fn nom_take_while_string(data: &str, input: char) -> nom::IResult<&str, String> {
    let (remaining, nommed) = take_while(|b| b == input)(data)?;
    Ok((remaining, nommed.to_string()))
}

#[op2]
#[serde]
/// Expose `take_while` bytes function to Deno
pub(crate) fn js_nom_take_while_bytes(
    #[buffer] data: JsBuffer,
    input: u8,
) -> Result<NomBytesJs, AnyError> {
    let results = nom_take_while_bytes(&data, input);
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(_) => return Err(anyhow!("Failed to nom while bytes")),
    };
    let nom_bytes = NomBytesJs {
        remaining: remaining.to_vec().into(),
        nommed: nommed.into(),
    };

    Ok(nom_bytes)
}

/// Expose `take_while` bytes function to Deno
fn nom_take_while_bytes(data: &[u8], input: u8) -> nom::IResult<&[u8], Vec<u8>> {
    let (remaining, nommed) = take_while(|b| b == input)(data)?;
    Ok((remaining, nommed.to_vec()))
}

#[cfg(test)]
mod tests {
    use crate::{
        runtime::{
            deno::execute_script,
            nom::parsers::{
                nom_take_bytes, nom_take_string, nom_take_until_bytes, nom_take_until_string,
                nom_take_while_bytes, nom_take_while_string,
            },
        },
        structs::artifacts::runtime::script::JSScript,
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
    fn test_js_nom_take_while_until() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbm9tL3BhcnNlcnMudHMKZnVuY3Rpb24gdGFrZShkYXRhLCBpbnB1dCkgewogIGlmIChpbnB1dCA8IDApIHsKICAgIGNvbnN0IGVyciA9IG5ldyBFcnJvcigicHJvdmlkZWQgbmVnYXRpdmUgbnVtYmVyIik7CiAgICByZXR1cm4gZXJyOwogIH0KICBpZiAodHlwZW9mIGRhdGEgPT09ICJzdHJpbmciKSB7CiAgICBjb25zdCByZXN1bHRfc3RyaW5nID0gRGVuby5jb3JlLm9wcy5qc19ub21fdGFrZV9zdHJpbmcoZGF0YSwgaW5wdXQpOwogICAgY29uc3Qgbm9tX3N0cmluZyA9IEpTT04ucGFyc2UocmVzdWx0X3N0cmluZyk7CiAgICByZXR1cm4gbm9tX3N0cmluZzsKICB9CiAgY29uc3QgcmVzdWx0ID0gRGVuby5jb3JlLm9wcy5qc19ub21fdGFrZV9ieXRlcyhkYXRhLCBpbnB1dCk7CiAgcmV0dXJuIHJlc3VsdDsKfQpmdW5jdGlvbiB0YWtlX3VudGlsKGRhdGEsIGlucHV0KSB7CiAgaWYgKHR5cGVvZiBkYXRhID09PSAic3RyaW5nIiAmJiB0eXBlb2YgaW5wdXQgPT09ICJzdHJpbmciKSB7CiAgICBjb25zdCByZXN1bHRfc3RyaW5nID0gRGVuby5jb3JlLm9wcy5qc19ub21fdGFrZV91bnRpbF9zdHJpbmcoCiAgICAgIGRhdGEsCiAgICAgIGlucHV0CiAgICApOwogICAgY29uc3Qgbm9tX3N0cmluZyA9IEpTT04ucGFyc2UocmVzdWx0X3N0cmluZyk7CiAgICBpZiAobm9tX3N0cmluZy5ub21tZWQubGVuZ3RoID09PSAwICYmIG5vbV9zdHJpbmcucmVtYWluaW5nLmxlbmd0aCA9PT0gMCkgewogICAgICByZXR1cm4gbmV3IEVycm9yKCJmYWlsZWQgdG8gcGFyc2Ugc3RyaW5nIGRhdGEiKTsKICAgIH0KICAgIHJldHVybiBub21fc3RyaW5nOwogIH0gZWxzZSBpZiAoZGF0YSBpbnN0YW5jZW9mIFVpbnQ4QXJyYXkgJiYgaW5wdXQgaW5zdGFuY2VvZiBVaW50OEFycmF5KSB7CiAgICBjb25zdCByZXN1bHQgPSBEZW5vLmNvcmUub3BzLmpzX25vbV90YWtlX3VudGlsX2J5dGVzKAogICAgICBkYXRhLAogICAgICBpbnB1dAogICAgKTsKICAgIGlmIChyZXN1bHQubm9tbWVkLmxlbmd0aCA9PT0gMCAmJiByZXN1bHQucmVtYWluaW5nLmxlbmd0aCA9PT0gMCkgewogICAgICByZXR1cm4gbmV3IEVycm9yKCJmYWlsZWQgdG8gcGFyc2UgYnl0ZXMgZGF0YSIpOwogICAgfQogICAgcmV0dXJuIHJlc3VsdDsKICB9CiAgcmV0dXJuIG5ldyBFcnJvcigicHJvdmlkZWQgdW5zdXBwb3J0ZWQgZGF0YSBhbmQvb3IgaW5wdXQgdHlwZXMiKTsKfQpmdW5jdGlvbiB0YWtlX3doaWxlKGRhdGEsIGlucHV0KSB7CiAgaWYgKHR5cGVvZiBpbnB1dCA9PT0gInN0cmluZyIgJiYgaW5wdXQubGVuZ3RoICE9IDEpIHsKICAgIGNvbnN0IGVyciA9IG5ldyBFcnJvcigicHJvdmlkZWQgc3RyaW5nIGxlbmd0aCBncmVhdGVyIHRoYW4gMSIpOwogICAgcmV0dXJuIGVycjsKICB9IGVsc2UgaWYgKHR5cGVvZiBpbnB1dCA9PT0gIm51bWJlciIgJiYgaW5wdXQgPCAwKSB7CiAgICBjb25zdCBlcnIgPSBuZXcgRXJyb3IoInByb3ZpZGVkIG5lZ2F0aXZlIG51bWJlciIpOwogICAgcmV0dXJuIGVycjsKICB9CiAgaWYgKHR5cGVvZiBkYXRhID09PSAic3RyaW5nIiAmJiB0eXBlb2YgaW5wdXQgPT09ICJzdHJpbmciKSB7CiAgICBjb25zdCByZXN1bHRfc3RyaW5nID0gRGVuby5jb3JlLm9wcy5qc19ub21fdGFrZV93aGlsZV9zdHJpbmcoCiAgICAgIGRhdGEsCiAgICAgIGlucHV0CiAgICApOwogICAgY29uc3Qgbm9tX3N0cmluZyA9IEpTT04ucGFyc2UocmVzdWx0X3N0cmluZyk7CiAgICBpZiAobm9tX3N0cmluZy5ub21tZWQubGVuZ3RoID09PSAwICYmIG5vbV9zdHJpbmcucmVtYWluaW5nLmxlbmd0aCA9PT0gMCkgewogICAgICByZXR1cm4gbmV3IEVycm9yKCJmYWlsZWQgdG8gcGFyc2Ugc3RyaW5nIGRhdGEiKTsKICAgIH0KICAgIHJldHVybiBub21fc3RyaW5nOwogIH0gZWxzZSBpZiAoZGF0YSBpbnN0YW5jZW9mIFVpbnQ4QXJyYXkgJiYgdHlwZW9mIGlucHV0ID09PSAibnVtYmVyIikgewogICAgY29uc3QgcmVzdWx0ID0gRGVuby5jb3JlLm9wcy5qc19ub21fdGFrZV93aGlsZV9ieXRlcygKICAgICAgZGF0YSwKICAgICAgaW5wdXQKICAgICk7CiAgICBpZiAocmVzdWx0Lm5vbW1lZC5sZW5ndGggPT09IDAgJiYgcmVzdWx0LnJlbWFpbmluZy5sZW5ndGggPT09IDApIHsKICAgICAgcmV0dXJuIG5ldyBFcnJvcigiZmFpbGVkIHRvIHBhcnNlIGJ5dGVzIGRhdGEiKTsKICAgIH0KICAgIHJldHVybiByZXN1bHQ7CiAgfQogIHJldHVybiBuZXcgRXJyb3IoInByb3ZpZGVkIHVuc3VwcG9ydGVkIGRhdGEgYW5kL29yIGlucHV0IHR5cGVzIik7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCByZXN1bHQgPSB0YWtlX3doaWxlKCJhYWFhYWNyYWIhIiwgImEiKTsKICBpZiAocmVzdWx0IGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYEZhaWxlZCB0byBub20gc3RyaW5nOiAke3Jlc3VsdH1gKTsKICAgIHJldHVybiByZXN1bHQ7CiAgfQogIGNvbnNvbGUubG9nKAogICAgYEkgbm9tbWVkOiAnJHtyZXN1bHQubm9tbWVkfScuIEkgaGF2ZSByZW1haW5pbmc6ICcke3Jlc3VsdC5yZW1haW5pbmd9J2AKICApOwogIGNvbnN0IGJ5dGVzID0gbmV3IFVpbnQ4QXJyYXkoWzAsIDAsIDAsIDAsIDFdKTsKICBjb25zdCByc3VsdEJ5dGVzID0gdGFrZV93aGlsZShieXRlcywgMCk7CiAgaWYgKHJzdWx0Qnl0ZXMgaW5zdGFuY2VvZiBFcnJvcikgewogICAgY29uc29sZS5lcnJvcihgRmFpbGVkIHRvIG5vbSBieXRlczogJHtyc3VsdEJ5dGVzLm1lc3NhZ2V9YCk7CiAgICByZXR1cm4gcmVzdWx0OwogIH0KICBjb25zb2xlLmxvZygKICAgIGBJIG5vbW1lZCBieXRlczogJHtyc3VsdEJ5dGVzLm5vbW1lZH0uIFN0cmluZyByZW1haW5pbmcgKHZpYSBieXRlcyB0byBzdHJpbmcpOiAke3JzdWx0Qnl0ZXMucmVtYWluaW5nfWAKICApOwogIGxldCB0ZXN0ID0gImEsc2ltcGxlLGNzdixzdHJpbmciOwogIHdoaWxlICh0ZXN0LmluY2x1ZGVzKCIsIikpIHsKICAgIGNvbnN0IHJlc3VsdDIgPSB0YWtlX3VudGlsKHRlc3QsICIsIik7CiAgICBpZiAocmVzdWx0MiBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICAgIGNvbnNvbGUuZXJyb3IoYEZhaWxlZCB0byBub20gc3RyaW5nOiAke3Jlc3VsdDJ9YCk7CiAgICAgIHJldHVybiByZXN1bHQyOwogICAgfQogICAgY29uc3QgcmVtYWluID0gdGFrZShyZXN1bHQyLnJlbWFpbmluZywgMSk7CiAgICBpZiAocmVtYWluIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgICAgY29uc29sZS5lcnJvcihgRmFpbGVkIHRvIG5vbSByZW1haW4gc3RyaW5nOiAke3Jlc3VsdDJ9YCk7CiAgICAgIHJldHVybiByZXN1bHQyOwogICAgfQogICAgdGVzdCA9IHJlbWFpbi5yZW1haW5pbmc7CiAgICBjb25zb2xlLmxvZyhyZXN1bHQyLm5vbW1lZCk7CiAgfQogIGNvbnN0IGJ5dGVzMiA9IG5ldyBVaW50OEFycmF5KFsxLCAwLCAwLCAxMywgMjIzXSk7CiAgY29uc3Qgc3RvcCA9IG5ldyBVaW50OEFycmF5KFsyMjNdKTsKICBjb25zdCByZXN1bHRCeXRlcyA9IHRha2VfdW50aWwoYnl0ZXMyLCBzdG9wKTsKICBpZiAocmVzdWx0Qnl0ZXMgaW5zdGFuY2VvZiBFcnJvcikgewogICAgY29uc29sZS5lcnJvcihgRmFpbGVkIHRvIG5vbSBieXRlcyB1bnRpbDogJHtyZXN1bHRCeXRlcy5tZXNzYWdlfWApOwogICAgcmV0dXJuIHJlc3VsdDsKICB9Cn0KbWFpbigpOwo=";
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

    #[test]
    fn test_nom_take_while_byts() {
        let test = [0, 0, 0, 1];
        nom_take_while_bytes(&test, 0).unwrap();
    }

    #[test]
    fn test_nom_take_while_string() {
        let test = "aaaab";
        nom_take_while_string(&test, 'a').unwrap();
    }

    #[test]
    fn test_nom_take_until_byts() {
        let test = [0, 0, 0, 1];
        nom_take_until_bytes(&test, &[1]).unwrap();
    }

    #[test]
    fn test_nom_take_until_string() {
        let test = "aaaab";
        nom_take_until_string(&test, "b").unwrap();
    }
}
