use super::accounts::{js_alt_users_windows, js_users_windows};
use boa_engine::{Context, JsString, NativeFunction};

/// Link Windows functions `BoaJS`
pub(crate) fn windows_functions(context: &mut Context) {
    let _ = context.register_global_callable(
        JsString::from("js_users_windows"),
        0,
        NativeFunction::from_fn_ptr(js_users_windows),
    );

    let _ = context.register_global_callable(
        JsString::from("js_alt_users_windows"),
        1,
        NativeFunction::from_fn_ptr(js_alt_users_windows),
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
