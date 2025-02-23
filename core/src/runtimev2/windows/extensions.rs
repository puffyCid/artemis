use super::{
    accounts::js_users_windows,
    amcache::js_amcache,
    bits::js_bits,
    ese::{js_filter_page_data, js_get_catalog, js_get_pages, js_get_table_columns, js_page_data},
    eventlogs::js_eventlogs,
    jumplists::js_jumplists,
    ntfs::{js_read_ads, js_read_raw_file},
    outlook::{js_read_attachment, js_read_folder, js_read_messages, js_root_folder},
    pe::js_get_pe,
    prefetch::js_prefetch,
    recyclebin::js_recycle_bin,
    registry::{js_registry, js_sk_info},
    search::js_search,
    services::js_services,
    shellbags::js_shellbags,
    shellitems::js_shellitems,
    shimcache::js_shimcache,
    shimdb::js_shimdb,
    shortcuts::js_lnk,
    srum::js_srum,
    tasks::js_tasks,
    userassist::js_userassist,
    usnjrnl::js_usnjrnl,
    wmi::js_wmipersist,
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

    let _ = context.register_global_callable(
        JsString::from("js_read_raw_file"),
        1,
        NativeFunction::from_fn_ptr(js_read_raw_file),
    );

    let _ = context.register_global_callable(
        JsString::from("js_read_ads"),
        2,
        NativeFunction::from_fn_ptr(js_read_ads),
    );

    let _ = context.register_global_callable(
        JsString::from("js_root_folder"),
        2,
        NativeFunction::from_fn_ptr(js_root_folder),
    );

    let _ = context.register_global_callable(
        JsString::from("js_read_folder"),
        3,
        NativeFunction::from_fn_ptr(js_read_folder),
    );

    let _ = context.register_global_callable(
        JsString::from("js_read_messages"),
        4,
        NativeFunction::from_fn_ptr(js_read_messages),
    );

    let _ = context.register_global_callable(
        JsString::from("js_read_attachment"),
        4,
        NativeFunction::from_fn_ptr(js_read_attachment),
    );

    let _ = context.register_global_callable(
        JsString::from("js_get_pe"),
        1,
        NativeFunction::from_fn_ptr(js_get_pe),
    );

    let _ = context.register_global_callable(
        JsString::from("js_prefetch"),
        1,
        NativeFunction::from_fn_ptr(js_prefetch),
    );

    let _ = context.register_global_callable(
        JsString::from("js_recycle_bin"),
        1,
        NativeFunction::from_fn_ptr(js_recycle_bin),
    );

    let _ = context.register_global_callable(
        JsString::from("js_registry"),
        1,
        NativeFunction::from_fn_ptr(js_registry),
    );

    let _ = context.register_global_callable(
        JsString::from("js_sk_info"),
        2,
        NativeFunction::from_fn_ptr(js_sk_info),
    );

    let _ = context.register_global_callable(
        JsString::from("js_search"),
        2,
        NativeFunction::from_fn_ptr(js_search),
    );

    let _ = context.register_global_callable(
        JsString::from("js_shellbags"),
        2,
        NativeFunction::from_fn_ptr(js_shellbags),
    );

    let _ = context.register_global_callable(
        JsString::from("js_services"),
        1,
        NativeFunction::from_fn_ptr(js_services),
    );

    let _ = context.register_global_callable(
        JsString::from("js_shellitems"),
        1,
        NativeFunction::from_fn_ptr(js_shellitems),
    );

    let _ = context.register_global_callable(
        JsString::from("js_shimcache"),
        1,
        NativeFunction::from_fn_ptr(js_shimcache),
    );

    let _ = context.register_global_callable(
        JsString::from("js_shimdb"),
        1,
        NativeFunction::from_fn_ptr(js_shimdb),
    );

    let _ = context.register_global_callable(
        JsString::from("js_lnk"),
        1,
        NativeFunction::from_fn_ptr(js_lnk),
    );

    let _ = context.register_global_callable(
        JsString::from("js_srum"),
        2,
        NativeFunction::from_fn_ptr(js_srum),
    );

    let _ = context.register_global_callable(
        JsString::from("js_tasks"),
        1,
        NativeFunction::from_fn_ptr(js_tasks),
    );

    let _ = context.register_global_callable(
        JsString::from("js_userassist"),
        2,
        NativeFunction::from_fn_ptr(js_userassist),
    );

    let _ = context.register_global_callable(
        JsString::from("js_usnjrnl"),
        2,
        NativeFunction::from_fn_ptr(js_usnjrnl),
    );

    let _ = context.register_global_callable(
        JsString::from("js_wmipersist"),
        2,
        NativeFunction::from_fn_ptr(js_wmipersist),
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
