use crate::runtime::encoding::{
    base64::{js_base64_decode, js_base64_encode},
    bytes::js_encode_bytes,
    strings::js_extract_utf8_string,
    xml::js_read_xml,
};
use deno_core::Op;

/// Link Rust encoding functions to `Deno core` to provide access to encoding/decoding functions
pub(crate) fn enocoding_runtime() -> Vec<deno_core::OpDecl> {
    vec![
        js_base64_decode::DECL,
        js_base64_encode::DECL,
        js_extract_utf8_string::DECL,
        js_encode_bytes::DECL,
        js_read_xml::DECL,
    ]
}
