use crate::runtime::applications::{
    chromium::{
        get_chromium_downloads, get_chromium_history, get_chromium_users_downloads,
        get_chromium_users_history,
    },
    firefox::{
        get_firefox_downloads, get_firefox_history, get_firefox_users_downloads,
        get_firefox_users_history,
    },
};
use deno_core::Op;

/// Link Rust functions to `Deno core`
pub(crate) fn app_functions() -> Vec<deno_core::OpDecl> {
    vec![
        get_firefox_users_history::DECL,
        get_firefox_history::DECL,
        get_firefox_users_downloads::DECL,
        get_firefox_downloads::DECL,
        get_chromium_users_history::DECL,
        get_chromium_history::DECL,
        get_chromium_users_downloads::DECL,
        get_chromium_downloads::DECL,
    ]
}

#[cfg(test)]
mod tests {
    use super::app_functions;

    #[test]
    fn test_app_functions() {
        let results = app_functions();
        assert!(results.len() > 5)
    }
}
