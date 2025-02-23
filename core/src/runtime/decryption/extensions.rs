use super::decrypt::js_decrypt_aes;
use boa_engine::{Context, JsString, NativeFunction};

/// Link Decryption functions `BoaJS`
pub(crate) fn decrypt_functions(context: &mut Context) {
    let _ = context.register_global_callable(
        JsString::from("js_decrypt_aes"),
        2,
        NativeFunction::from_fn_ptr(js_decrypt_aes),
    );
}

#[cfg(test)]
mod tests {
    use super::decrypt_functions;
    use boa_engine::Context;

    #[test]
    fn test_decrypt_functions() {
        let mut context = Context::default();
        decrypt_functions(&mut context);
    }
}
