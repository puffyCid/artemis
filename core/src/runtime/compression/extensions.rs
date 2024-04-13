use crate::runtime::compression::decompress::js_decompress_zlib;

/// Link Rust compression functions to `Deno core`
pub(crate) fn compression_functions() -> Vec<deno_core::OpDecl> {
    vec![js_decompress_zlib()]
}

#[cfg(test)]
mod tests {
    use super::compression_functions;

    #[test]
    fn test_compression_functions() {
        let results = compression_functions();
        assert!(results.len() >= 1)
    }
}
