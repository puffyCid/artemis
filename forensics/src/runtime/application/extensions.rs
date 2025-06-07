use super::sqlite::js_query_sqlite;
use boa_engine::{Context, JsString, NativeFunction};

/// Link application functions `BoaJS`
pub(crate) fn application_functions(context: &mut Context) {
    let _ = context.register_global_callable(
        JsString::from("js_query_sqlite"),
        2,
        NativeFunction::from_fn_ptr(js_query_sqlite),
    );
}

#[cfg(test)]
mod tests {
    use super::application_functions;
    use boa_engine::Context;

    #[test]
    fn test_application_functions() {
        let mut context = Context::default();
        application_functions(&mut context);
    }
}
