use super::{
    accounts::js_users_windows,
    amcache::js_amcache,
    bits::js_bits,
    ese::{js_filter_page_data, js_get_catalog, js_get_pages, js_get_table_columns, js_page_data},
    eventlogs::js_eventlogs,
    jumplists::js_jumplists,
};
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

    let _ = context.register_global_callable(
        JsString::from("js_get_catalog"),
        1,
        NativeFunction::from_fn_ptr(js_get_catalog),
    );

    let _ = context.register_global_callable(
        JsString::from("js_get_pages"),
        2,
        NativeFunction::from_fn_ptr(js_get_pages),
    );

    let _ = context.register_global_callable(
        JsString::from("js_page_data"),
        2,
        NativeFunction::from_fn_ptr(js_page_data),
    );

    let _ = context.register_global_callable(
        JsString::from("js_filter_page_data"),
        6,
        NativeFunction::from_fn_ptr(js_filter_page_data),
    );

    let _ = context.register_global_callable(
        JsString::from("js_get_table_columns"),
        5,
        NativeFunction::from_fn_ptr(js_get_table_columns),
    );

    let _ = context.register_global_callable(
        JsString::from("js_eventlogs"),
        5,
        NativeFunction::from_fn_ptr(js_eventlogs),
    );

    let _ = context.register_global_callable(
        JsString::from("js_jumplists"),
        1,
        NativeFunction::from_fn_ptr(js_jumplists),
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
