use super::env::{js_env, js_env_value};
use boa_engine::{Context, JsString, NativeFunction};

/// Link Environment functions `BoaJS`
pub(crate) fn env_functions(context: &mut Context) {
    let _ = context.register_global_callable(
        JsString::from("js_env"),
        2,
        NativeFunction::from_fn_ptr(js_env),
    );

    let _ = context.register_global_callable(
        JsString::from("js_env_value"),
        2,
        NativeFunction::from_fn_ptr(js_env_value),
    );
}

#[cfg(test)]
mod tests {
    use super::env_functions;
    use boa_engine::Context;

    #[test]
    fn test_env_functions() {
        let mut context = Context::default();
        env_functions(&mut context);
    }
}
