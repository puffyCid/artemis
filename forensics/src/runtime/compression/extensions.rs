use super::decompress::{
    js_decompress_gzip, js_decompress_snappy, js_decompress_zlib, js_decompress_zstd,
};
use boa_engine::{Context, JsString, NativeFunction};

/// Link Decompression functions `BoaJS`
pub(crate) fn decompress_functions(context: &mut Context) {
    let _ = context.register_global_callable(
        JsString::from("js_decompress_zlib"),
        3,
        NativeFunction::from_fn_ptr(js_decompress_zlib),
    );

    let _ = context.register_global_callable(
        JsString::from("js_decompress_gzip"),
        1,
        NativeFunction::from_fn_ptr(js_decompress_gzip),
    );

    let _ = context.register_global_callable(
        JsString::from("js_decompress_snappy"),
        1,
        NativeFunction::from_fn_ptr(js_decompress_snappy),
    );

    let _ = context.register_global_callable(
        JsString::from("js_decompress_zstd"),
        1,
        NativeFunction::from_fn_ptr(js_decompress_zstd),
    );
}

#[cfg(test)]
mod tests {
    use super::decompress_functions;
    use boa_engine::Context;

    #[test]
    fn test_decompress_functions() {
        let mut context = Context::default();
        decompress_functions(&mut context);
    }
}
