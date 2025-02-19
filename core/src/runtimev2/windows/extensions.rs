use super::{accounts::js_users_windows, amcache::js_amcache, bits::js_bits};
use boa_engine::{Context, JsString, NativeFunction};

/// Link Windows functions `BoaJS`
pub(crate) fn windows_functions(context: &mut Context) {
    let _ = context.register_global_callable(
        JsString::from("js_users_windows"),
        1,
        NativeFunction::from_fn_ptr(js_users_windows),
    );

    let _ = context.register_global_callable(
        JsString::from("js_amcache"),
        1,
        NativeFunction::from_fn_ptr(js_amcache),
    );

    let _ = context.register_global_callable(
        JsString::from("js_bits"),
        1,
        NativeFunction::from_fn_ptr(js_bits),
    );
}

#[cfg(test)]
mod tests {
    use super::windows_functions;
    use boa_engine::Context;

    #[test]
    fn test_windows_functions() {
        let mut context = Context::default();
        windows_functions(&mut context);
    }
}
