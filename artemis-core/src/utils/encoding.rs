use base64::{engine::general_purpose, DecodeError, Engine};

/// Base64 encode data use the STANDARD engine (alphabet along with "+" and "/")
pub(crate) fn base64_encode_standard(data: &[u8]) -> String {
    general_purpose::STANDARD.encode(data)
}

/// Base64 decode data use the STANDARD engine (alphabet along with "+" and "/")
pub(crate) fn base64_decode_standard(data: &str) -> Result<Vec<u8>, DecodeError> {
    general_purpose::STANDARD.decode(data)
}

#[cfg(test)]
mod tests {
    use super::{base64_decode_standard, base64_encode_standard};

    #[test]
    fn test_base64_encode_standard() {
        let test = b"Hello word!";
        let result = base64_encode_standard(test);
        assert_eq!(result, "SGVsbG8gd29yZCE=")
    }

    #[test]
    fn test_base64_decode_standard() {
        let test = "SGVsbG8gd29yZCE=";
        let result = base64_decode_standard(test).unwrap();
        assert_eq!(result, b"Hello word!")
    }
}
