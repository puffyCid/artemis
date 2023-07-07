use crate::runtime::unix::{
    cron::get_cron,
    shellhistory::{get_bash_history, get_python_history, get_zsh_history},
    sudo::get_sudologs,
};

/// Link Rust functions to `Deno core`
pub(crate) fn unix_functions() -> Vec<deno_core::OpDecl> {
    vec![
        get_cron::decl(),
        get_bash_history::decl(),
        get_zsh_history::decl(),
        get_python_history::decl(),
        get_sudologs::decl(),
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
