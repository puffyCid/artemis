use super::{
    base64::{js_base64_decode, js_base64_encode},
    bytes::js_encode_bytes,
    protobuf::js_parse_protobuf,
    strings::{js_bytes_to_hex_string, js_extract_utf16_string, js_extract_utf8_string},
    uuid::{js_format_guid_be_bytes, js_format_guid_le_bytes, js_generate_uuid},
};
use boa_engine::{Context, JsString, NativeFunction};

/// Link encoding functions `BoaJS`
pub(crate) fn encoding_functions(context: &mut Context) {
    let _ = context.register_global_callable(
        JsString::from("js_base64_decode"),
        1,
        NativeFunction::from_fn_ptr(js_base64_decode),
    );

    let _ = context.register_global_callable(
        JsString::from("js_base64_encode"),
        1,
        NativeFunction::from_fn_ptr(js_base64_encode),
    );

    let _ = context.register_global_callable(
        JsString::from("js_encode_bytes"),
        1,
        NativeFunction::from_fn_ptr(js_encode_bytes),
    );

    let _ = context.register_global_callable(
        JsString::from("js_parse_protobuf"),
        1,
        NativeFunction::from_fn_ptr(js_parse_protobuf),
    );

    let _ = context.register_global_callable(
        JsString::from("js_extract_utf8_string"),
        1,
        NativeFunction::from_fn_ptr(js_extract_utf8_string),
    );

    let _ = context.register_global_callable(
        JsString::from("js_extract_utf16_string"),
        1,
        NativeFunction::from_fn_ptr(js_extract_utf16_string),
    );

    let _ = context.register_global_callable(
        JsString::from("js_bytes_to_hex_string"),
        1,
        NativeFunction::from_fn_ptr(js_bytes_to_hex_string),
    );

    let _ = context.register_global_callable(
        JsString::from("js_format_guid_le_bytes"),
        1,
        NativeFunction::from_fn_ptr(js_format_guid_le_bytes),
    );

    let _ = context.register_global_callable(
        JsString::from("js_format_guid_be_bytes"),
        1,
        NativeFunction::from_fn_ptr(js_format_guid_be_bytes),
    );

    let _ = context.register_global_callable(
        JsString::from("js_generate_uuid"),
        0,
        NativeFunction::from_fn_ptr(js_generate_uuid),
    );
}

#[cfg(test)]
mod tests {
    use super::encoding_functions;
    use boa_engine::Context;

    #[test]
    fn test_encoding_functions() {
        let mut context = Context::default();
        encoding_functions(&mut context);
    }
}
