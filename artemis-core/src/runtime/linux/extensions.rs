use super::logons::get_logon;
use super::sudo::get_sudologs;
use crate::runtime::linux::{executable::get_elf, journal::get_journal};
use deno_core::{Extension, Op};

/// Include all the `Artemis` function in the `Runtime`
pub(crate) fn setup_linux_extensions() -> Vec<Extension> {
    let extensions = Extension {
        name: "artemis",
        ops: grab_functions().into(),
        ..Default::default()
    };
    vec![extensions]
}

/// Link Rust functions to `Deno core`
fn grab_functions() -> Vec<deno_core::OpDecl> {
    let exts = vec![
        get_elf::DECL,
        get_journal::DECL,
        get_logon::DECL,
        get_sudologs::DECL,
    ];

    exts
}

#[cfg(test)]
mod tests {
    use super::{grab_functions, setup_linux_extensions};

    #[test]
    fn test_grab_functions() {
        let results = grab_functions();
        assert!(results.len() > 2);
    }

    #[test]
    fn test_setup_linux_extensions() {
        let results = setup_linux_extensions();
        assert_eq!(results.len(), 1);
    }
}
