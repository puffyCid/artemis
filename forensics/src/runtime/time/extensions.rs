use super::conversion::{
    js_cocoatime_to_unixepoch, js_fat_time_to_unixepoch, js_filetime_to_unixepoch,
    js_hfs_to_unixepoch, js_ole_automationtime_to_unixepoch, js_time_now,
    js_webkit_time_to_unixepoch,
};
use boa_engine::{Context, JsString, NativeFunction};

/// Link time functions `BoaJS`
pub(crate) fn time_functions(context: &mut Context) {
    let _ = context.register_global_callable(
        JsString::from("js_time_now"),
        0,
        NativeFunction::from_fn_ptr(js_time_now),
    );

    let _ = context.register_global_callable(
        JsString::from("js_filetime_to_unixepoch"),
        1,
        NativeFunction::from_fn_ptr(js_filetime_to_unixepoch),
    );

    let _ = context.register_global_callable(
        JsString::from("js_cocoatime_to_unixepoch"),
        1,
        NativeFunction::from_fn_ptr(js_cocoatime_to_unixepoch),
    );

    let _ = context.register_global_callable(
        JsString::from("js_hfs_to_unixepoch"),
        1,
        NativeFunction::from_fn_ptr(js_hfs_to_unixepoch),
    );

    let _ = context.register_global_callable(
        JsString::from("js_ole_automationtime_to_unixepoch"),
        1,
        NativeFunction::from_fn_ptr(js_ole_automationtime_to_unixepoch),
    );

    let _ = context.register_global_callable(
        JsString::from("js_webkit_time_to_unixepoch"),
        1,
        NativeFunction::from_fn_ptr(js_webkit_time_to_unixepoch),
    );

    let _ = context.register_global_callable(
        JsString::from("js_fat_time_to_unixepoch"),
        1,
        NativeFunction::from_fn_ptr(js_fat_time_to_unixepoch),
    );
}

#[cfg(test)]
mod tests {
    use super::time_functions;
    use boa_engine::Context;

    #[test]
    fn test_time_functions() {
        let mut context = Context::default();
        time_functions(&mut context);
    }
}
