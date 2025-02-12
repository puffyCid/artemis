use super::{
    base64::{js_base64_decode, js_base64_encode},
    bytes::js_encode_bytes,
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
