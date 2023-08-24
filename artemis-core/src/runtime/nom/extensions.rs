use crate::runtime::nom::parsers::{js_nom_take_bytes, js_nom_take_string};
use deno_core::Op;

/// Link nom functions to `Deno core`
pub(crate) fn nom_functions() -> Vec<deno_core::OpDecl> {
    vec![js_nom_take_string::DECL, js_nom_take_bytes::DECL]
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
