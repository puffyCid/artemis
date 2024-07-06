use crate::runtime::decryption::decrypt::js_decrypt_aes;

/// Link Rust decryption functions to `Deno core`
pub(crate) fn decryption_functions() -> Vec<deno_core::OpDecl> {
    vec![js_decrypt_aes()]
}

#[cfg(test)]
mod tests {
    use super::decryption_functions;

    #[test]
    fn test_decryption_functions() {
        let results = decryption_functions();
        assert!(results.len() >= 1)
    }
}
