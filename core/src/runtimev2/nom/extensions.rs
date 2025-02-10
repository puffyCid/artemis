use crate::runtimev2::nom::helpers::js_nom_unsigned_four_bytes;
use boa_engine::{Context, JsString, NativeFunction};

/// Link nom functions to `Deno core`
pub(crate) fn nom_functions(context: &mut Context) {
    let _ = context.register_global_callable(
        JsString::from("js_nom_unsigned_four_bytes"),
        2,
        NativeFunction::from_fn_ptr(js_nom_unsigned_four_bytes),
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
