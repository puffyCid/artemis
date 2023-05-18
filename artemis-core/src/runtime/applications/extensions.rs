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

/// Link Rust functions to `Deno core`
pub(crate) fn app_functions() -> Vec<deno_core::OpDecl> {
    vec![
        get_firefox_users_history::decl(),
        get_firefox_history::decl(),
        get_firefox_users_downloads::decl(),
        get_firefox_downloads::decl(),
        get_chromium_users_history::decl(),
        get_chromium_history::decl(),
        get_chromium_users_downloads::decl(),
        get_chromium_downloads::decl(),
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
