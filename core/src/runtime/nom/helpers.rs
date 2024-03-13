use crate::utils::nom_helper::{
    nom_signed_eight_bytes, nom_signed_four_bytes, nom_signed_two_bytes, nom_unsigned_eight_bytes,
    nom_unsigned_four_bytes, nom_unsigned_one_byte, nom_unsigned_sixteen_bytes,
    nom_unsigned_two_bytes, Endian,
};
use deno_core::{anyhow::anyhow, error::AnyError, op2, JsBuffer, ToJsBuffer};
use serde::Serialize;

#[derive(Serialize)]
pub(crate) struct NomUnsignedJs {
    remaining: ToJsBuffer,
    value: usize,
}

#[op2]
#[serde]
/// Expose nom helper to Deno
pub(crate) fn js_nom_unsigned_four_bytes(
    #[buffer] data: JsBuffer,
    big_little: u8,
) -> Result<NomUnsignedJs, AnyError> {
    let endian = if big_little == 1 {
        Endian::Le
    } else {
        Endian::Be
    };

    let results = nom_unsigned_four_bytes(&data, endian);
    let (remaining, nommed) = match results {
        Ok(result) => result,
        Err(_) => return Err(anyhow!("Failed to nom unsigned four bytes")),
    };
    let nom_bytes = NomUnsignedJs {
        remaining: remaining.to_vec().into(),
        value: nommed as usize,
    };

    Ok(nom_bytes)
}

#[op2]
#[serde]
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

#[op2]
#[serde]
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

#[op2]
#[serde]
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
#[op2]
#[serde]
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

#[op2]
#[serde]
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

#[op2]
#[serde]
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

#[op2]
#[serde]
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
    fn test_js_nom_helpers() {
        let test = "Ly8gaHR0cHM6Ly9yYXcuZ2l0aHVidXNlcmNvbnRlbnQuY29tL3B1ZmZ5Y2lkL2FydGVtaXMtYXBpL21hc3Rlci9zcmMvbm9tL2hlbHBlcnMudHMKZnVuY3Rpb24gbm9tX3Vuc2lnbmVkX2ZvdXJfYnl0ZXMoZGF0YSwgZW5kaWFuZXNzKSB7CiAgY29uc3QgcmVzdWx0ID0gRGVuby5jb3JlLm9wcy5qc19ub21fdW5zaWduZWRfZm91cl9ieXRlcygKICAgIGRhdGEsCiAgICBlbmRpYW5lc3MKICApOwogIGlmIChyZXN1bHQucmVtYWluaW5nLmxlbmd0aCA9PT0gMCAmJiByZXN1bHQudmFsdWUgPT09IDApIHsKICAgIHJldHVybiBuZXcgRXJyb3IoIm5vbW1lZCB6ZXJvIGJ5dGVzIik7CiAgfQogIHJldHVybiByZXN1bHQ7Cn0KZnVuY3Rpb24gbm9tX3Vuc2lnbmVkX2VpZ2h0X2J5dGVzKGRhdGEsIGVuZGlhbmVzcykgewogIGNvbnN0IHJlc3VsdCA9IERlbm8uY29yZS5vcHMuanNfbm9tX3Vuc2lnbmVkX2VpZ2h0X2J5dGVzKAogICAgZGF0YSwKICAgIGVuZGlhbmVzcwogICk7CiAgaWYgKHJlc3VsdC5yZW1haW5pbmcubGVuZ3RoID09PSAwICYmIHJlc3VsdC52YWx1ZSA9PT0gMCkgewogICAgcmV0dXJuIG5ldyBFcnJvcigibm9tbWVkIHplcm8gYnl0ZXMiKTsKICB9CiAgcmV0dXJuIHJlc3VsdDsKfQpmdW5jdGlvbiBub21fdW5zaWduZWRfdHdvX2J5dGVzKGRhdGEsIGVuZGlhbmVzcykgewogIGNvbnN0IHJlc3VsdCA9IERlbm8uY29yZS5vcHMuanNfbm9tX3Vuc2lnbmVkX3R3b19ieXRlcygKICAgIGRhdGEsCiAgICBlbmRpYW5lc3MKICApOwogIGlmIChyZXN1bHQucmVtYWluaW5nLmxlbmd0aCA9PT0gMCAmJiByZXN1bHQudmFsdWUgPT09IDApIHsKICAgIHJldHVybiBuZXcgRXJyb3IoIm5vbW1lZCB6ZXJvIGJ5dGVzIik7CiAgfQogIHJldHVybiByZXN1bHQ7Cn0KZnVuY3Rpb24gbm9tX3Vuc2lnbmVkX29uZV9ieXRlcyhkYXRhLCBlbmRpYW5lc3MpIHsKICBjb25zdCByZXN1bHQgPSBEZW5vLmNvcmUub3BzLmpzX25vbV91bnNpZ25lZF9vbmVfYnl0ZXMoCiAgICBkYXRhLAogICAgZW5kaWFuZXNzCiAgKTsKICBpZiAocmVzdWx0LnJlbWFpbmluZy5sZW5ndGggPT09IDAgJiYgcmVzdWx0LnZhbHVlID09PSAwKSB7CiAgICByZXR1cm4gbmV3IEVycm9yKCJub21tZWQgemVybyBieXRlcyIpOwogIH0KICByZXR1cm4gcmVzdWx0Owp9CmZ1bmN0aW9uIG5vbV91bnNpZ25lZF9zaXh0ZWVuX2J5dGVzKGRhdGEsIGVuZGlhbmVzcykgewogIGNvbnN0IHJlc3VsdCA9IERlbm8uY29yZS5vcHMuanNfbm9tX3Vuc2lnbmVkX3NpeHRlZW5fYnl0ZXMoCiAgICBkYXRhLAogICAgZW5kaWFuZXNzCiAgKTsKICBpZiAocmVzdWx0LnJlbWFpbmluZy5sZW5ndGggPT09IDAgJiYgcmVzdWx0LnZhbHVlID09PSAiIikgewogICAgcmV0dXJuIG5ldyBFcnJvcigibm9tbWVkIHplcm8gYnl0ZXMiKTsKICB9CiAgcmV0dXJuIHJlc3VsdDsKfQpmdW5jdGlvbiBub21fc2lnbmVkX2ZvdXJfYnl0ZXMoZGF0YSwgZW5kaWFuZXNzKSB7CiAgY29uc3QgcmVzdWx0ID0gRGVuby5jb3JlLm9wcy5qc19ub21fc2lnbmVkX2ZvdXJfYnl0ZXMoCiAgICBkYXRhLAogICAgZW5kaWFuZXNzCiAgKTsKICBpZiAocmVzdWx0LnJlbWFpbmluZy5sZW5ndGggPT09IDAgJiYgcmVzdWx0LnZhbHVlID09PSAwKSB7CiAgICByZXR1cm4gbmV3IEVycm9yKCJub21tZWQgemVybyBieXRlcyIpOwogIH0KICByZXR1cm4gcmVzdWx0Owp9CmZ1bmN0aW9uIG5vbV9zaWduZWRfZWlnaHRfYnl0ZXMoZGF0YSwgZW5kaWFuZXNzKSB7CiAgY29uc3QgcmVzdWx0ID0gRGVuby5jb3JlLm9wcy5qc19ub21fc2lnbmVkX2VpZ2h0X2J5dGVzKAogICAgZGF0YSwKICAgIGVuZGlhbmVzcwogICk7CiAgaWYgKHJlc3VsdC5yZW1haW5pbmcubGVuZ3RoID09PSAwICYmIHJlc3VsdC52YWx1ZSA9PT0gMCkgewogICAgcmV0dXJuIG5ldyBFcnJvcigibm9tbWVkIHplcm8gYnl0ZXMiKTsKICB9CiAgcmV0dXJuIHJlc3VsdDsKfQpmdW5jdGlvbiBub21fc2lnbmVkX3R3b19ieXRlcyhkYXRhLCBlbmRpYW5lc3MpIHsKICBjb25zdCByZXN1bHQgPSBEZW5vLmNvcmUub3BzLmpzX25vbV9zaWduZWRfdHdvX2J5dGVzKAogICAgZGF0YSwKICAgIGVuZGlhbmVzcwogICk7CiAgaWYgKHJlc3VsdC5yZW1haW5pbmcubGVuZ3RoID09PSAwICYmIHJlc3VsdC52YWx1ZSA9PT0gMCkgewogICAgcmV0dXJuIG5ldyBFcnJvcigibm9tbWVkIHplcm8gYnl0ZXMiKTsKICB9CiAgcmV0dXJuIHJlc3VsdDsKfQoKLy8gbWFpbi50cwpmdW5jdGlvbiBtYWluKCkgewogIGNvbnN0IGJ5dGVzID0gbmV3IFVpbnQ4QXJyYXkoWzEsIDIsIDM0LCA4LCAxLCAyLCAzNCwgOCwgMSwgMiwgMzQsIDgsIDEsIDIsIDM0LCA4LCAxLCAyLCAzNCwgOF0pOwogIGxldCByZXN1bHQgPSBub21fdW5zaWduZWRfb25lX2J5dGVzKGJ5dGVzLCAxIC8qIExlICovKTsKICBpZiAocmVzdWx0IGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYEZhaWxlZCB0byBub20gYnl0ZXM6ICR7cmVzdWx0Lm1lc3NhZ2V9YCk7CiAgICByZXR1cm4gcmVzdWx0OwogIH0KICByZXN1bHQgPSBub21fdW5zaWduZWRfdHdvX2J5dGVzKGJ5dGVzLCAxIC8qIExlICovKTsKICBpZiAocmVzdWx0IGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYEZhaWxlZCB0byBub20gYnl0ZXM6ICR7cmVzdWx0Lm1lc3NhZ2V9YCk7CiAgICByZXR1cm4gcmVzdWx0OwogIH0KICByZXN1bHQgPSBub21fdW5zaWduZWRfZm91cl9ieXRlcyhieXRlcywgMSAvKiBMZSAqLyk7CiAgaWYgKHJlc3VsdCBpbnN0YW5jZW9mIEVycm9yKSB7CiAgICBjb25zb2xlLmVycm9yKGBGYWlsZWQgdG8gbm9tIGJ5dGVzOiAke3Jlc3VsdC5tZXNzYWdlfWApOwogICAgcmV0dXJuIHJlc3VsdDsKICB9CiAgcmVzdWx0ID0gbm9tX3Vuc2lnbmVkX2VpZ2h0X2J5dGVzKGJ5dGVzLCAxIC8qIExlICovKTsKICBpZiAocmVzdWx0IGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYEZhaWxlZCB0byBub20gYnl0ZXM6ICR7cmVzdWx0Lm1lc3NhZ2V9YCk7CiAgICByZXR1cm4gcmVzdWx0OwogIH0KICBjb25zdCBsYXJnZSA9IG5vbV91bnNpZ25lZF9zaXh0ZWVuX2J5dGVzKGJ5dGVzLCAxIC8qIExlICovKTsKICBpZiAobGFyZ2UgaW5zdGFuY2VvZiBFcnJvcikgewogICAgY29uc29sZS5lcnJvcihgRmFpbGVkIHRvIG5vbSBieXRlczogJHtsYXJnZS5tZXNzYWdlfWApOwogICAgcmV0dXJuIGxhcmdlOwogIH0KICByZXN1bHQgPSBub21fc2lnbmVkX2VpZ2h0X2J5dGVzKGJ5dGVzLCAxIC8qIExlICovKTsKICBpZiAocmVzdWx0IGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYEZhaWxlZCB0byBub20gYnl0ZXM6ICR7cmVzdWx0Lm1lc3NhZ2V9YCk7CiAgICByZXR1cm4gcmVzdWx0OwogIH0KICByZXN1bHQgPSBub21fc2lnbmVkX2ZvdXJfYnl0ZXMoYnl0ZXMsIDEgLyogTGUgKi8pOwogIGlmIChyZXN1bHQgaW5zdGFuY2VvZiBFcnJvcikgewogICAgY29uc29sZS5lcnJvcihgRmFpbGVkIHRvIG5vbSBieXRlczogJHtyZXN1bHQubWVzc2FnZX1gKTsKICAgIHJldHVybiByZXN1bHQ7CiAgfQogIHJlc3VsdCA9IG5vbV9zaWduZWRfdHdvX2J5dGVzKGJ5dGVzLCAxIC8qIExlICovKTsKICBpZiAocmVzdWx0IGluc3RhbmNlb2YgRXJyb3IpIHsKICAgIGNvbnNvbGUuZXJyb3IoYEZhaWxlZCB0byBub20gYnl0ZXM6ICR7cmVzdWx0Lm1lc3NhZ2V9YCk7CiAgICByZXR1cm4gcmVzdWx0OwogIH0KfQptYWluKCk7Cg==";
        let mut output = output_options("runtime_test", "local", "./tmp", false);
        let script = JSScript {
            name: String::from("helpers"),
            script: test.to_string(),
        };
        execute_script(&mut output, &script).unwrap();
    }
}
