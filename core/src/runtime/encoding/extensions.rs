use crate::runtime::encoding::{
    base64::{js_base64_decode, js_base64_encode},
    bytes::js_encode_bytes,
    strings::{js_bytes_to_hex_string, js_extract_utf16_string, js_extract_utf8_string},
    uuid::{js_format_guid_be_bytes, js_format_guid_le_bytes, js_generate_uuid},
    xml::js_read_xml,
};
use deno_core::Op;

/// Link Rust encoding functions to `Deno core` to provide access to encoding/decoding functions
pub(crate) fn enocoding_functions() -> Vec<deno_core::OpDecl> {
    vec![
        js_base64_decode::DECL,
        js_base64_encode::DECL,
        js_extract_utf8_string::DECL,
        js_extract_utf16_string::DECL,
        js_encode_bytes::DECL,
        js_read_xml::DECL,
        js_bytes_to_hex_string::DECL,
        js_format_guid_be_bytes::DECL,
        js_format_guid_le_bytes::DECL,
        js_generate_uuid::DECL,
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
