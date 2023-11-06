use crate::runtime::http::client::js_request;
use deno_core::Op;

/// Link HTTP networking functions to `Deno core`
pub(crate) fn http_functions() -> Vec<deno_core::OpDecl> {
    vec![js_request::DECL]
}

#[cfg(test)]
mod tests {
    use super::http_functions;

    #[test]
    fn test_system_functions() {
        let results = http_functions();
        assert!(results.len() > 0)
    }
}
