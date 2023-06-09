use crate::runtime::applications::extensions::app_functions;
use crate::runtime::linux::{executable::get_elf, journal::get_journal};
use crate::runtime::system::extensions::system_functions;
use crate::runtime::unix::extensions::unix_functions;
use deno_core::Extension;

/// Include all the `Artemis` function in the `Runtime`
pub(crate) fn setup_extensions() -> Vec<Extension> {
    let extensions = Extension::builder("artemis").ops(grab_functions()).build();
    vec![extensions]
}

/// Link Rust functions to `Deno core`
fn grab_functions() -> Vec<deno_core::OpDecl> {
    let mut exts = vec![get_elf::decl(), get_journal::decl()];

    exts.append(&mut app_functions());
    exts.append(&mut unix_functions());
    exts.append(&mut system_functions());
    exts
}

#[cfg(test)]
mod tests {
    use super::{grab_functions, setup_extensions};

    #[test]
    fn test_grab_functions() {
        let results = grab_functions();
        assert!(results.len() > 2);
    }

    #[test]
    fn test_setup_extensions() {
        let results = setup_extensions();
        assert_eq!(results.len(), 1);
    }
}
