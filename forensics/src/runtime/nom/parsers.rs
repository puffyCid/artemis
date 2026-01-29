use crate::runtime::helper::{bytes_arg, number_arg, string_arg};
use boa_engine::{Context, JsError, JsResult, JsValue, js_string, object::builtins::JsUint8Array};
use nom::bytes::complete::{take_until, take_while};
use serde::Serialize;

#[derive(Serialize)]
struct NomStringJs {
    remaining: String,
    nommed: String,
}

/// Expose `take_until` string function to Boa
pub(crate) fn js_nom_take_until_string(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let data = string_arg(args, 0)?;
    let input = string_arg(args, 1)?;

    let results = nom_take_until_string(&data, &input);
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to take until: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let nom_string = NomStringJs {
        remaining: remaining.to_string(),
        nommed,
    };
    let results = serde_json::to_value(&nom_string).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Expose `take_until` string function to Boa
fn nom_take_until_string<'a>(data: &'a str, input: &str) -> nom::IResult<&'a str, String> {
    let (remaining, nommed) = take_until(input)(data)?;
    Ok((remaining, nommed.to_string()))
}

/// Expose `take_until` bytes function to Boa
pub(crate) fn js_nom_take_until_bytes(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let data = bytes_arg(args, 0, context)?;
    let input = bytes_arg(args, 1, context)?;

    let results = nom_take_until_bytes(&data, &input);
    let (_remaining, nommed) = match results {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to take until: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let bytes = JsUint8Array::from_iter(nommed, context)?;

    Ok(bytes.into())
}

/// Expose `take_until` bytes function to Boa
fn nom_take_until_bytes<'a>(data: &'a [u8], input: &[u8]) -> nom::IResult<&'a [u8], Vec<u8>> {
    let (remaining, nommed) = take_until(input)(data)?;
    Ok((remaining, nommed.to_vec()))
}

/// Expose `take_while` string function to Boa
pub(crate) fn js_nom_take_while_string(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let data = string_arg(args, 0)?;
    let input = string_arg(args, 1)?;

    let results = nom_take_while_string(&data, input.chars().next().unwrap_or_default());
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to take while: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let nom_string = NomStringJs {
        remaining: remaining.to_string(),
        nommed,
    };
    let results = serde_json::to_value(&nom_string).unwrap_or_default();
    let value = JsValue::from_json(&results, context)?;

    Ok(value)
}

/// Expose `take_while` string function to Boa
fn nom_take_while_string(data: &str, input: char) -> nom::IResult<&str, String> {
    let (remaining, nommed) = take_while(|b| b == input)(data)?;
    Ok((remaining, nommed.to_string()))
}

/// Expose `take_while` bytes function to Boa
pub(crate) fn js_nom_take_while_bytes(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let data = bytes_arg(args, 0, context)?;
    let input = number_arg(args, 1)? as u8;

    let results = nom_take_while_bytes(&data, input);
    let (_remaining, nommed) = match results {
        Ok(result) => result,
        Err(err) => {
            let issue = format!("Failed to take while: {err:?}");
            return Err(JsError::from_opaque(js_string!(issue).into()));
        }
    };
    let bytes = JsUint8Array::from_iter(nommed, context)?;

    Ok(bytes.into())
}

/// Expose `take_while` bytes function to Boa
fn nom_take_while_bytes(data: &[u8], input: u8) -> nom::IResult<&[u8], Vec<u8>> {
    let (remaining, nommed) = take_while(|b| b == input)(data)?;
    Ok((remaining, nommed.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::{
        nom_take_until_bytes, nom_take_until_string, nom_take_while_bytes, nom_take_while_string,
    };
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
            endpoint_id: String::from("abcd"),
            output: output.to_string(),
            ..Default::default()
        }
    }

    #[test]
    fn test_js_nom_take_while_until() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbm9tL3BhcnNlcnMudHMKZnVuY3Rpb24gdGFrZV91bnRpbChkYXRhLCBpbnB1dCkgewogIGlmICh0eXBlb2YgZGF0YSA9PT0gInN0cmluZyIgJiYgdHlwZW9mIGlucHV0ID09PSAic3RyaW5nIikgewogICAgY29uc3QgcmVzdWx0ID0ganNfbm9tX3Rha2VfdW50aWxfc3RyaW5nKAogICAgICBkYXRhLAogICAgICBpbnB1dAogICAgKTsKICAgIGlmIChyZXN1bHQubm9tbWVkLmxlbmd0aCA9PT0gMCAmJiByZXN1bHQucmVtYWluaW5nLmxlbmd0aCA9PT0gMCkgewogICAgICByZXR1cm4gbmV3IEVycm9yKCJmYWlsZWQgdG8gcGFyc2Ugc3RyaW5nIGRhdGEiKTsKICAgIH0KICAgIHJldHVybiByZXN1bHQ7CiAgfSBlbHNlIGlmIChkYXRhIGluc3RhbmNlb2YgVWludDhBcnJheSAmJiBpbnB1dCBpbnN0YW5jZW9mIFVpbnQ4QXJyYXkpIHsKICAgIGNvbnN0IHJlc3VsdCA9IGpzX25vbV90YWtlX3VudGlsX2J5dGVzKAogICAgICBkYXRhLAogICAgICBpbnB1dAogICAgKTsKICAgIGlmIChyZXN1bHQubGVuZ3RoID09PSAwKSB7CiAgICAgIHJldHVybiBuZXcgRXJyb3IoImZhaWxlZCB0byBwYXJzZSBieXRlcyBkYXRhIik7CiAgICB9CiAgICByZXR1cm4gcmVzdWx0OwogIH0KICByZXR1cm4gbmV3IEVycm9yKCJwcm92aWRlZCB1bnN1cHBvcnRlZCBkYXRhIGFuZC9vciBpbnB1dCB0eXBlcyIpOwp9CmZ1bmN0aW9uIHRha2Vfd2hpbGUoZGF0YSwgaW5wdXQpIHsKICBpZiAodHlwZW9mIGlucHV0ID09PSAic3RyaW5nIiAmJiBpbnB1dC5sZW5ndGggIT0gMSkgewogICAgY29uc3QgZXJyID0gbmV3IEVycm9yKCJwcm92aWRlZCBzdHJpbmcgbGVuZ3RoIGdyZWF0ZXIgdGhhbiAxIik7CiAgICByZXR1cm4gZXJyOwogIH0gZWxzZSBpZiAodHlwZW9mIGlucHV0ID09PSAibnVtYmVyIiAmJiBpbnB1dCA8IDApIHsKICAgIGNvbnN0IGVyciA9IG5ldyBFcnJvcigicHJvdmlkZWQgbmVnYXRpdmUgbnVtYmVyIik7CiAgICByZXR1cm4gZXJyOwogIH0KICBpZiAodHlwZW9mIGRhdGEgPT09ICJzdHJpbmciICYmIHR5cGVvZiBpbnB1dCA9PT0gInN0cmluZyIpIHsKICAgIGNvbnN0IHJlc3VsdCA9IGpzX25vbV90YWtlX3doaWxlX3N0cmluZygKICAgICAgZGF0YSwKICAgICAgaW5wdXQKICAgICk7CiAgICBpZiAocmVzdWx0Lm5vbW1lZC5sZW5ndGggPT09IDAgJiYgcmVzdWx0LnJlbWFpbmluZy5sZW5ndGggPT09IDApIHsKICAgICAgcmV0dXJuIG5ldyBFcnJvcigiZmFpbGVkIHRvIHBhcnNlIHN0cmluZyBkYXRhIik7CiAgICB9CiAgICByZXR1cm4gcmVzdWx0OwogIH0gZWxzZSBpZiAoZGF0YSBpbnN0YW5jZW9mIFVpbnQ4QXJyYXkgJiYgdHlwZW9mIGlucHV0ID09PSAibnVtYmVyIikgewogICAgY29uc3QgcmVzdWx0ID0ganNfbm9tX3Rha2Vfd2hpbGVfYnl0ZXMoCiAgICAgIGRhdGEsCiAgICAgIGlucHV0CiAgICApOwogICAgaWYgKHJlc3VsdC5sZW5ndGggPT09IDApIHsKICAgICAgcmV0dXJuIG5ldyBFcnJvcigiZmFpbGVkIHRvIHBhcnNlIGJ5dGVzIGRhdGEiKTsKICAgIH0KICAgIHJldHVybiByZXN1bHQ7CiAgfQogIHJldHVybiBuZXcgRXJyb3IoInByb3ZpZGVkIHVuc3VwcG9ydGVkIGRhdGEgYW5kL29yIGlucHV0IHR5cGVzIik7Cn0KCi8vIG1haW4udHMKZnVuY3Rpb24gbWFpbigpIHsKICBjb25zdCByZXN1bHQgPSB0YWtlX3doaWxlKCJhYWFhYWNyYWIhIiwgImEiKTsKICBpZiAocmVzdWx0IGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYEZhaWxlZCB0byBub20gc3RyaW5nOiAke3Jlc3VsdH1gKTsKICAgIHJldHVybiByZXN1bHQ7CiAgfQogIGNvbnNvbGUubG9nKAogICAgYEkgbm9tbWVkOiAnJHtyZXN1bHQubm9tbWVkfScuIEkgaGF2ZSByZW1haW5pbmc6ICcke3Jlc3VsdC5yZW1haW5pbmd9J2AKICApOwogIGNvbnN0IGJ5dGVzID0gbmV3IFVpbnQ4QXJyYXkoWzAsIDAsIDAsIDAsIDFdKTsKICBjb25zdCByc3VsdEJ5dGVzID0gdGFrZV93aGlsZShieXRlcywgMCk7CiAgaWYgKHJzdWx0Qnl0ZXMgaW5zdGFuY2VvZiBFcnJvcikgewogICAgY29uc29sZS5lcnJvcihgRmFpbGVkIHRvIG5vbSBieXRlczogJHtyc3VsdEJ5dGVzfWApOwogICAgcmV0dXJuIHJlc3VsdDsKICB9CiAgY29uc29sZS5sb2coCiAgICBgSSBub21tZWQgYnl0ZXM6ICR7cnN1bHRCeXRlc30uYAogICk7CiAgbGV0IHRlc3QgPSAiYSxzaW1wbGUsY3N2LHN0cmluZyI7CiAgd2hpbGUgKHRlc3QuaW5jbHVkZXMoIiwiKSkgewogICAgY29uc3QgcmVzdWx0MiA9IHRha2VfdW50aWwodGVzdCwgIiwiKTsKICAgIGlmIChyZXN1bHQyIGluc3RhbmNlb2YgRXJyb3IpIHsKICAgICAgY29uc29sZS5lcnJvcihgRmFpbGVkIHRvIG5vbSBzdHJpbmc6ICR7cmVzdWx0Mn1gKTsKICAgICAgcmV0dXJuIHJlc3VsdDI7CiAgICB9CiAgICBjb25zb2xlLmxvZyhKU09OLnN0cmluZ2lmeShyZXN1bHQyKSk7CiAgICBicmVhazsKICB9CiAgY29uc3QgYnl0ZXMyID0gbmV3IFVpbnQ4QXJyYXkoWzEsIDAsIDAsIDEzLCAyMjNdKTsKICBjb25zdCBzdG9wID0gbmV3IFVpbnQ4QXJyYXkoWzIyM10pOwogIGNvbnN0IHJlc3VsdEJ5dGVzID0gdGFrZV91bnRpbChieXRlczIsIHN0b3ApOwogIGlmIChyZXN1bHRCeXRlcyBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICBjb25zb2xlLmVycm9yKGBGYWlsZWQgdG8gbm9tIGJ5dGVzIHVudGlsOiAke3Jlc3VsdEJ5dGVzfWApOwogICAgcmV0dXJuIHJlc3VsdDsKICB9Cn0KbWFpbigpOw==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("nomming"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }

    #[test]
    fn test_nom_take_while_bytes() {
        let test = [0, 0, 0, 1];
        nom_take_while_bytes(&test, 0).unwrap();
    }

    #[test]
    fn test_nom_take_while_string() {
        let test = "aaaab";
        nom_take_while_string(&test, 'a').unwrap();
    }

    #[test]
    fn test_nom_take_until_bytes() {
        let test = [0, 0, 0, 1];
        nom_take_until_bytes(&test, &[1]).unwrap();
    }

    #[test]
    fn test_nom_take_until_string() {
        let test = "aaaab";
        nom_take_until_string(&test, "b").unwrap();
    }
}
