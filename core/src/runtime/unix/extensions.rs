use super::{
    cron::js_get_cron,
    shellhistory::{js_bash_history, js_python_history, js_zsh_history},
};
use boa_engine::{Context, JsString, NativeFunction};

/// Link Unix functions `BoaJS`
pub(crate) fn unix_functions(context: &mut Context) {
    let _ = context.register_global_callable(
        JsString::from("js_get_cron"),
        0,
        NativeFunction::from_fn_ptr(js_get_cron),
    );

    let _ = context.register_global_callable(
        JsString::from("js_bash_history"),
        0,
        NativeFunction::from_fn_ptr(js_bash_history),
    );

    let _ = context.register_global_callable(
        JsString::from("js_zsh_history"),
        0,
        NativeFunction::from_fn_ptr(js_zsh_history),
    );

    let _ = context.register_global_callable(
        JsString::from("js_python_history"),
        0,
        NativeFunction::from_fn_ptr(js_python_history),
    );
}

#[cfg(test)]
mod tests {
    use super::unix_functions;
    use boa_engine::Context;

    #[test]
    fn test_unix_functions() {
        let mut context = Context::default();
        unix_functions(&mut context);
    }
}
