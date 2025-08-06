use super::{
    acquire::js_acquire_file,
    directory::js_read_dir,
    files::{js_glob, js_hash_file, js_read_file, js_read_lines, js_read_text_file, js_stat},
};
use boa_engine::{Context, JsString, NativeFunction};

/// Link filesystem functions `BoaJS`
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
        4,
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

    let _ = context.register_global_callable(
        JsString::from("js_acquire_file"),
        2,
        NativeFunction::from_fn_ptr(js_acquire_file),
    );

    let _ = context.register_global_callable(
        JsString::from("js_read_dir"),
        2,
        NativeFunction::from_fn_ptr(js_read_dir),
    );

    let _ = context.register_global_callable(
        JsString::from("js_read_lines"),
        3,
        NativeFunction::from_fn_ptr(js_read_lines),
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
