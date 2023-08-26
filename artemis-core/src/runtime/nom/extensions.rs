use crate::runtime::nom::{
    helpers::{
        js_nom_signed_eight_bytes, js_nom_signed_four_bytes, js_nom_signed_two_bytes,
        js_nom_unsigned_eight_bytes, js_nom_unsigned_four_bytes, js_nom_unsigned_one_bytes,
        js_nom_unsigned_sixteen_bytes, js_nom_unsigned_two_bytes,
    },
    parsers::{
        js_nom_take_bytes, js_nom_take_string, js_nom_take_until_bytes, js_nom_take_until_string,
        js_nom_take_while_bytes, js_nom_take_while_string,
    },
};
use deno_core::Op;

/// Link nom functions to `Deno core`
pub(crate) fn nom_functions() -> Vec<deno_core::OpDecl> {
    vec![
        js_nom_take_string::DECL,
        js_nom_take_bytes::DECL,
        js_nom_take_while_string::DECL,
        js_nom_take_while_bytes::DECL,
        js_nom_take_until_bytes::DECL,
        js_nom_take_until_string::DECL,
        js_nom_signed_eight_bytes::DECL,
        js_nom_signed_four_bytes::DECL,
        js_nom_signed_two_bytes::DECL,
        js_nom_unsigned_eight_bytes::DECL,
        js_nom_unsigned_four_bytes::DECL,
        js_nom_unsigned_one_bytes::DECL,
        js_nom_unsigned_sixteen_bytes::DECL,
        js_nom_unsigned_two_bytes::DECL,
    ]
}

#[cfg(test)]
mod tests {
    use super::nom_functions;

    #[test]
    fn test_system_functions() {
        let results = nom_functions();
        assert!(results.len() > 1)
    }
}
