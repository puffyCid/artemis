use super::{
    elf::js_get_elf, journal::js_get_journal, logons::js_get_logon, sudo::js_get_sudologs_linux,
};
use boa_engine::{Context, JsString, NativeFunction};

/// Link Linux functions `BoaJS`
pub(crate) fn linux_functions(context: &mut Context) {
    let _ = context.register_global_callable(
        JsString::from("js_get_elf"),
        1,
        NativeFunction::from_fn_ptr(js_get_elf),
    );

    let _ = context.register_global_callable(
        JsString::from("js_get_journal"),
        1,
        NativeFunction::from_fn_ptr(js_get_journal),
    );

    let _ = context.register_global_callable(
        JsString::from("js_get_logon"),
        1,
        NativeFunction::from_fn_ptr(js_get_logon),
    );

    let _ = context.register_global_callable(
        JsString::from("js_get_sudologs_linux"),
        1,
        NativeFunction::from_fn_ptr(js_get_sudologs_linux),
    );
}

#[cfg(test)]
mod tests {
    use super::linux_functions;
    use boa_engine::Context;

    #[test]
    fn test_linux_functions() {
        let mut context = Context::default();
        linux_functions(&mut context);
    }
}
