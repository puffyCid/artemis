use crate::runtime::unix::{
    cron::get_cron,
    shellhistory::{get_bash_history, get_python_history, get_zsh_history},
};
use deno_core::Op;

/// Link Rust functions to `Deno core`
pub(crate) fn unix_functions() -> Vec<deno_core::OpDecl> {
    vec![
        get_cron::DECL,
        get_bash_history::DECL,
        get_zsh_history::DECL,
        get_python_history::DECL,
    ]
}

#[cfg(test)]
mod tests {
    use super::unix_functions;

    #[test]
    fn test_unix_functions() {
        let results = unix_functions();
        assert!(results.len() > 3)
    }
}
