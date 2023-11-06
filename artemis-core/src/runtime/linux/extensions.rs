use super::logons::get_logon;
use crate::runtime::linux::{executable::get_elf, journal::get_journal};
use crate::runtime::{
    applications::extensions::app_functions, encoding::extensions::enocoding_runtime,
    environment::extensions::env_runtime, filesystem::extensions::fs_runtime,
    http::extensions::http_functions, nom::extensions::nom_functions,
    system::extensions::system_functions, time::extensions::time_functions,
    unix::extensions::unix_functions,
};
use deno_core::{Extension, Op};

/// Include all the `Artemis` function in the `Runtime`
pub(crate) fn setup_extensions() -> Vec<Extension> {
    let extensions = Extension {
        name: "artemis",
        ops: grab_functions().into(),
        ..Default::default()
    };
    vec![extensions]
}

/// Link Rust functions to `Deno core`
fn grab_functions() -> Vec<deno_core::OpDecl> {
    let mut exts = vec![get_elf::DECL, get_journal::DECL, get_logon::DECL];

    exts.append(&mut app_functions());
    exts.append(&mut unix_functions());
    exts.append(&mut system_functions());

    exts.append(&mut fs_runtime());
    exts.append(&mut env_runtime());
    exts.append(&mut enocoding_runtime());

    exts.append(&mut nom_functions());
    exts.append(&mut time_functions());
    exts.append(&mut http_functions());

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
