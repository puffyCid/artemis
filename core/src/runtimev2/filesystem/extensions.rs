use super::files::{js_glob, js_hash_file, js_read_file, js_read_text_file, js_stat};
use boa_engine::{Context, JsString, NativeFunction};

/// Link filesystem functions BoaJS
pub(crate) fn filesystem_functions(context: &mut Context) {
    let _ = context.register_global_callable(
        JsString::from("js_stat"),
        1,
        NativeFunction::from_fn_ptr(js_stat),
    );

    let _ = context.register_global_callable(
        JsString::from("js_glob"),
        1,
        NativeFunction::from_fn_ptr(js_glob),
    );

    let _ = context.register_global_callable(
        JsString::from("js_hash_file"),
        1,
        NativeFunction::from_fn_ptr(js_hash_file),
    );

    let _ = context.register_global_callable(
        JsString::from("js_read_text_file"),
        1,
        NativeFunction::from_fn_ptr(js_read_text_file),
    );

    let _ = context.register_global_callable(
        JsString::from("js_read_file"),
        1,
        NativeFunction::from_fn_ptr(js_read_file),
    );
}

#[cfg(test)]
mod tests {
    use super::filesystem_functions;
    use boa_engine::Context;

    #[test]
    fn test_filesystem_functions() {
        let mut context = Context::default();
        filesystem_functions(&mut context);
    }
}
