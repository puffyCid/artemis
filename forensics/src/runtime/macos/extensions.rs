use super::{
    accounts::{js_groups_macos, js_users_macos},
    bookmarks::js_bookmark,
    emond::js_emond,
    execpolicy::js_execpolicy,
    fsevents::js_fsevents,
    launchd::{js_launchd_agents, js_launchd_daemons},
    loginitems::js_loginitems,
    macho::js_macho,
    plist::{js_plist, js_plist_data},
    spotlight::{js_setup_spotlight_parser, js_spotlight},
    sudo::js_sudologs_macos,
    unifiedlog::js_unified_log,
};
use boa_engine::{Context, JsString, NativeFunction};

/// Link macOS functions `BoaJS`
pub(crate) fn macos_functions(context: &mut Context) {
    let _ = context.register_global_callable(
        JsString::from("js_users_macos"),
        1,
        NativeFunction::from_fn_ptr(js_users_macos),
    );

    let _ = context.register_global_callable(
        JsString::from("js_groups_macos"),
        1,
        NativeFunction::from_fn_ptr(js_groups_macos),
    );

    let _ = context.register_global_callable(
        JsString::from("js_bookmark"),
        1,
        NativeFunction::from_fn_ptr(js_bookmark),
    );

    let _ = context.register_global_callable(
        JsString::from("js_emond"),
        1,
        NativeFunction::from_fn_ptr(js_emond),
    );

    let _ = context.register_global_callable(
        JsString::from("js_launchd_daemons"),
        0,
        NativeFunction::from_fn_ptr(js_launchd_daemons),
    );

    let _ = context.register_global_callable(
        JsString::from("js_launchd_agents"),
        0,
        NativeFunction::from_fn_ptr(js_launchd_agents),
    );

    let _ = context.register_global_callable(
        JsString::from("js_execpolicy"),
        1,
        NativeFunction::from_fn_ptr(js_execpolicy),
    );

    let _ = context.register_global_callable(
        JsString::from("js_fsevents"),
        1,
        NativeFunction::from_fn_ptr(js_fsevents),
    );

    let _ = context.register_global_callable(
        JsString::from("js_macho"),
        1,
        NativeFunction::from_fn_ptr(js_macho),
    );

    let _ = context.register_global_callable(
        JsString::from("js_plist"),
        1,
        NativeFunction::from_fn_ptr(js_plist),
    );

    let _ = context.register_global_callable(
        JsString::from("js_plist_data"),
        1,
        NativeFunction::from_fn_ptr(js_plist_data),
    );

    let _ = context.register_global_callable(
        JsString::from("js_loginitems"),
        1,
        NativeFunction::from_fn_ptr(js_loginitems),
    );

    let _ = context.register_global_callable(
        JsString::from("js_spotlight"),
        3,
        NativeFunction::from_fn_ptr(js_spotlight),
    );

    let _ = context.register_global_callable(
        JsString::from("js_setup_spotlight_parser"),
        1,
        NativeFunction::from_fn_ptr(js_setup_spotlight_parser),
    );

    let _ = context.register_global_callable(
        JsString::from("js_sudologs_macos"),
        1,
        NativeFunction::from_fn_ptr(js_sudologs_macos),
    );
    let _ = context.register_global_callable(
        JsString::from("js_unified_log"),
        2,
        NativeFunction::from_fn_ptr(js_unified_log),
    );
}

#[cfg(test)]
mod tests {
    use super::macos_functions;
    use boa_engine::Context;

    #[test]
    fn test_macos_functions() {
        let mut context = Context::default();
        macos_functions(&mut context);
    }
}
