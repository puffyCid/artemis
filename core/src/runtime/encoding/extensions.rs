use crate::runtime::encoding::{
    base64::{js_base64_decode, js_base64_encode},
    bytes::js_encode_bytes,
    strings::{js_bytes_to_hex_string, js_extract_utf16_string, js_extract_utf8_string},
    uuid::{js_format_guid_be_bytes, js_format_guid_le_bytes, js_generate_uuid},
    xml::js_read_xml,
};

/// Link Rust encoding functions to `Deno core` to provide access to encoding/decoding functions
pub(crate) fn enocoding_functions() -> Vec<deno_core::OpDecl> {
    vec![
        js_base64_decode(),
        js_base64_encode(),
        js_extract_utf8_string(),
        js_extract_utf16_string(),
        js_encode_bytes(),
        js_read_xml(),
        js_bytes_to_hex_string(),
        js_format_guid_be_bytes(),
        js_format_guid_le_bytes(),
        js_generate_uuid(),
    ]
}

#[cfg(test)]
mod tests {
    use super::enocoding_functions;

    #[test]
    fn test_enocoding_functions() {
        let results = enocoding_functions();
        assert!(results.len() > 1)
    }
}
