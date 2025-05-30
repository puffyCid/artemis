use base64::{DecodeError, Engine, engine::general_purpose};

/// Base64 encode data using the STANDARD engine (alphabet along with "+" and "/")
pub(crate) fn base64_encode_standard(data: &[u8]) -> String {
    general_purpose::STANDARD.encode(data)
}

/// Base64 decode data use the STANDARD engine (alphabet along with "+" and "/")
pub(crate) fn base64_decode_standard(data: &str) -> Result<Vec<u8>, DecodeError> {
    general_purpose::STANDARD.decode(data)
}
