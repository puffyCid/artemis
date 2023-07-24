use crate::utils::encoding::base64_decode_standard;
use deno_core::{error::AnyError, op};

#[op]
/// Decode Base64 data
fn js_base64_decode(data: String) -> Result<Vec<u8>, AnyError> {
    let decoded_data = base64_decode_standard(&data)?;

    Ok(decoded_data)
}

#[cfg(test)]
mod tests {}
