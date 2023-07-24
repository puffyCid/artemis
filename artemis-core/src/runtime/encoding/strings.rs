use crate::utils::strings::extract_utf8_string;
use deno_core::op;

#[op]
/// Attempt to extract a UTF8 string from raw bytes
fn js_extract_utf8_string(data: Vec<u8>) -> String {
    extract_utf8_string(&data)
}

#[cfg(test)]
mod tests {}
