use crate::runtime::encoding::{base64::js_base64_decode, strings::js_extract_utf8_string};
use deno_core::Op;

/// Link Rust encoding functions to `Deno core` to provide access to encoding/decoding functions
pub(crate) fn enocoding_runtime() -> Vec<deno_core::OpDecl> {
    vec![js_base64_decode::DECL, js_extract_utf8_string::DECL]
}
