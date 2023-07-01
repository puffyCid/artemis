use crate::runtime::system::{
    output::output_results, processes::get_processes, systeminfo::get_systeminfo,
};

/// Link Rust functions to `Deno core`
pub(crate) fn system_functions() -> Vec<deno_core::OpDecl> {
    vec![
        get_processes::decl(),
        get_systeminfo::decl(),
        output_results::decl(),
    ]
}

#[cfg(test)]
mod tests {
    use super::system_functions;

    #[test]
    fn test_system_functions() {
        let results = system_functions();
        assert!(results.len() > 1)
    }
}
