use crate::utils::nom_helper::{nom_unsigned_four_bytes, Endian};
use boa_engine::{
    js_string,
    object::builtins::{JsArrayBuffer, JsUint8Array},
    Context, JsArgs, JsError, JsObject, JsResult, JsValue,
};
use log::error;
use serde::Serialize;
use serde_json::Value;

#[derive(Serialize)]
pub(crate) struct NomUnsignedJs {
    remaining: Vec<u8>,
    value: usize,
}

/// Expose nom helper to Deno
pub(crate) fn js_nom_unsigned_four_bytes(
    _this: &JsValue,
    args: &[JsValue],
    context: &mut Context,
) -> JsResult<JsValue> {
    let big_little = args.get_or_undefined(1);
    let endian;
    if !big_little.is_integer() {
        endian = Endian::Le;
    } else {
        if big_little.to_int8(context).is_ok_and(|x| x == 1) {
            endian = Endian::Le;
        } else {
            endian = Endian::Be;
        }
    }

    let bytes = match args.get(0) {
        Some(result) => result,
        None => {
            error!("[runtime] Could not get bytes got no arg");
            return Err(JsError::from_opaque(
                js_string!("Failed to get bytes from args").into(),
            ));
        }
    };
    let array_value =
        JsUint8Array::from_object(bytes.as_object().unwrap_or(&JsObject::default()).clone())?;
    let data = JsArrayBuffer::from_object(
        array_value
            .buffer(context)?
            .as_object()
            .unwrap_or(&JsObject::default())
            .clone(),
    )?;

    let binding = match data.data() {
        Some(result) => result,
        None => {
            return Err(JsError::from_opaque(
                js_string!("Failed to get buffer from args").into(),
            ));
        }
    };
    let results = nom_unsigned_four_bytes(&binding, endian);
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(_) => {
            return Err(JsError::from_opaque(
                js_string!("Failed to nom four bytes").into(),
            ));
        }
    };
    let buff = JsArrayBuffer::from_byte_block(remaining.to_vec(), context)?;
    //let remaining = JsUint8Array::from_array_buffer(buff, context)?;
    let nom_bytes = NomUnsignedJs {
        remaining: buff.detach(&JsValue::undefined())?,
        value: nommed as usize,
    };

    let nom_serde = serde_json::to_value(&nom_bytes).unwrap_or(Value::Null);

    Ok(JsValue::from_json(&nom_serde, context)?)
}

/*
/// Expose nom helper to Deno
pub(crate) fn js_nom_unsigned_eight_bytes(
    #[buffer] data: JsBuffer,
    big_little: u8,
) -> Result<NomUnsignedJs, AnyError> {
    let endian = if big_little == 1 {
        Endian::Le
    } else {
        Endian::Be
    };

    let results = nom_unsigned_eight_bytes(&data, endian);
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(_) => return Err(anyhow!("Failed to nom unsigned eight bytes")),
    };
    let nom_bytes = NomUnsignedJs {
        remaining: remaining.to_vec().into(),
        value: nommed as usize,
    };

    Ok(nom_bytes)
}

/// Expose nom helper to Deno
pub(crate) fn js_nom_unsigned_two_bytes(
    #[buffer] data: JsBuffer,
    big_little: u8,
) -> Result<NomUnsignedJs, AnyError> {
    let endian = if big_little == 1 {
        Endian::Le
    } else {
        Endian::Be
    };

    let results = nom_unsigned_two_bytes(&data, endian);
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(_) => return Err(anyhow!("Failed to nom unsigned two bytes")),
    };
    let nom_bytes = NomUnsignedJs {
        remaining: remaining.to_vec().into(),
        value: nommed as usize,
    };

    Ok(nom_bytes)
}

/// Expose nom helper to Deno
pub(crate) fn js_nom_unsigned_one_bytes(
    #[buffer] data: JsBuffer,
    big_little: u8,
) -> Result<NomUnsignedJs, AnyError> {
    let endian = if big_little == 1 {
        Endian::Le
    } else {
        Endian::Be
    };

    let results = nom_unsigned_one_byte(&data, endian);
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(_) => return Err(anyhow!("Failed to nom unsigned one bytes")),
    };
    let nom_bytes = NomUnsignedJs {
        remaining: remaining.to_vec().into(),
        value: nommed as usize,
    };

    Ok(nom_bytes)
}

#[derive(Serialize)]
pub(crate) struct NomUnsignedLargeJs {
    remaining: ToJsBuffer,
    value: String,
}

/// Expose nom helper to Deno
pub(crate) fn js_nom_unsigned_sixteen_bytes(
    #[buffer] data: JsBuffer,
    big_little: u8,
) -> Result<NomUnsignedLargeJs, AnyError> {
    let endian = if big_little == 1 {
        Endian::Le
    } else {
        Endian::Be
    };

    let results = nom_unsigned_sixteen_bytes(&data, endian);
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(_) => return Err(anyhow!("Failed to nom unsigned sixteen bytes")),
    };
    let nom_bytes = NomUnsignedLargeJs {
        remaining: remaining.to_vec().into(),
        value: nommed.to_string(),
    };

    Ok(nom_bytes)
}

#[derive(Serialize)]
pub(crate) struct NomSignedJs {
    remaining: ToJsBuffer,
    value: isize,
}

/// Expose nom helper to Deno
pub(crate) fn js_nom_signed_four_bytes(
    #[buffer] data: JsBuffer,
    big_little: u8,
) -> Result<NomSignedJs, AnyError> {
    let endian = if big_little == 1 {
        Endian::Le
    } else {
        Endian::Be
    };

    let results = nom_signed_four_bytes(&data, endian);
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(_) => return Err(anyhow!("Failed to nom signed four bytes")),
    };
    let nom_bytes = NomSignedJs {
        remaining: remaining.to_vec().into(),
        value: nommed as isize,
    };

    Ok(nom_bytes)
}

/// Expose nom helper to Deno
pub(crate) fn js_nom_signed_eight_bytes(
    #[buffer] data: JsBuffer,
    big_little: u8,
) -> Result<NomSignedJs, AnyError> {
    let endian = if big_little == 1 {
        Endian::Le
    } else {
        Endian::Be
    };

    let results = nom_signed_eight_bytes(&data, endian);
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(_) => return Err(anyhow!("Failed to nom signed eight bytes")),
    };
    let nom_bytes = NomSignedJs {
        remaining: remaining.to_vec().into(),
        value: nommed as isize,
    };

    Ok(nom_bytes)
}

/// Expose nom helper to Deno
pub(crate) fn js_nom_signed_two_bytes(
    #[buffer] data: JsBuffer,
    big_little: u8,
) -> Result<NomSignedJs, AnyError> {
    let endian = if big_little == 1 {
        Endian::Le
    } else {
        Endian::Be
    };

    let results = nom_signed_two_bytes(&data, endian);
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(_) => return Err(anyhow!("Failed to nom signed two bytes")),
    };
    let nom_bytes = NomSignedJs {
        remaining: remaining.to_vec().into(),
        value: nommed as isize,
    };

    Ok(nom_bytes)
}
*/
#[cfg(test)]
mod tests {
    use crate::{
        runtimev2::run::execute_script,
        structs::{artifacts::runtime::script::JSScript, toml::Output},
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
    fn test_js_nom_helpers() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbm9tL2hlbHBlcnMudHMKZnVuY3Rpb24gbm9tX3Vuc2lnbmVkX2ZvdXJfYnl0ZXMoZGF0YSwgZW5kaWFuZXNzKSB7CiAgY29uc3QgcmVzdWx0ID0ganNfbm9tX3Vuc2lnbmVkX2ZvdXJfYnl0ZXMoCiAgICBkYXRhLAogICAgZW5kaWFuZXNzCiAgKTsKICBpZiAocmVzdWx0LnJlbWFpbmluZy5sZW5ndGggPT09IDAgJiYgcmVzdWx0LnZhbHVlID09PSAwKSB7CiAgICByZXR1cm4gbmV3IEVycm9yKCJub21tZWQgemVybyBieXRlcyIpOwogIH0KICByZXR1cm4gcmVzdWx0Owp9CgoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGJ5dGVzID0gbmV3IFVpbnQ4QXJyYXkoWzEsIDIsIDM0LCA4LCAxLCAyLCAzNCwgOCwgMSwgMiwgMzQsIDgsIDEsIDIsIDM0LCA4LCAxLCAyLCAzNCwgOF0pOwogIGxldCByZXN1bHQgPSBub21fdW5zaWduZWRfZm91cl9ieXRlcyhieXRlcywgMSAvKiBMZSAqLyk7CiAgaWYgKHJlc3VsdCBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICBjb25zb2xlLmVycm9yKGBGYWlsZWQgdG8gbm9tIGJ5dGVzOiAke3Jlc3VsdC5tZXNzYWdlfWApOwogICAgcmV0dXJuIHJlc3VsdDsKICB9CmNvbnNvbGUubG9nKHJlc3VsdCk7CiAgCn0KbWFpbigpOwo=";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("helpers"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
