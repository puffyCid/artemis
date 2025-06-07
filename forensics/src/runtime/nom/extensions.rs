use super::parsers::{
    js_nom_take_until_bytes, js_nom_take_until_string, js_nom_take_while_bytes,
    js_nom_take_while_string,
};
use boa_engine::{Context, JsString, NativeFunction};

/// Link nom functions to `Boa core`
pub(crate) fn nom_functions(context: &mut Context) {
    let _ = context.register_global_callable(
        JsString::from("js_nom_take_until_string"),
        2,
        NativeFunction::from_fn_ptr(js_nom_take_until_string),
    );

    let _ = context.register_global_callable(
        JsString::from("js_nom_take_until_bytes"),
        2,
        NativeFunction::from_fn_ptr(js_nom_take_until_bytes),
    );

    let _ = context.register_global_callable(
        JsString::from("js_nom_take_while_string"),
        2,
        NativeFunction::from_fn_ptr(js_nom_take_while_string),
    );

    let _ = context.register_global_callable(
        JsString::from("js_nom_take_while_bytes"),
        2,
        NativeFunction::from_fn_ptr(js_nom_take_while_bytes),
    );
}

#[cfg(test)]
mod tests {
    use boa_engine::Context;

    use super::nom_functions;

    #[test]
    fn test_system_functions() {
        let mut context = Context::default();
        nom_functions(&mut context);
    }
}
