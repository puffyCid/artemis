use super::{client::js_request, url::js_url_parse};
use boa_engine::{Context, JsString, NativeFunction};

/// Link HTTP functions `BoaJS`
pub(crate) fn http_functions(context: &mut Context) {
    let _ = context.register_global_callable(
        JsString::from("js_request"),
        2,
        NativeFunction::from_fn_ptr(js_request),
    );

    let _ = context.register_global_callable(
        JsString::from("js_url_parse"),
        2,
        NativeFunction::from_fn_ptr(js_url_parse),
    );
}

#[cfg(test)]
mod tests {
    use super::http_functions;
    use boa_engine::Context;

    #[test]
    fn test_http_functions() {
        let mut context = Context::default();
        http_functions(&mut context);
    }
}
