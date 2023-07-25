use crate::utils::encoding::{base64_decode_standard, base64_encode_standard};
use deno_core::{error::AnyError, op};

#[op]
/// Decode Base64 data
fn js_base64_decode(data: String) -> Result<Vec<u8>, AnyError> {
    let decoded_data = base64_decode_standard(&data)?;

    Ok(decoded_data)
}

#[op]
/// Encode bytes to Base64 string
fn js_base64_encode(data: Vec<u8>) -> String {
    base64_encode_standard(&data)
}

#[cfg(test)]
mod tests {}
