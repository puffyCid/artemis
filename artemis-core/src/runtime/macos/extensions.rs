use super::{
    accounts::{get_groups, get_users},
    emond::get_emond,
    execpolicy::get_execpolicy,
    fsevents::get_fsevents,
    launchd::{get_launchd_agents, get_launchd_daemons},
    loginitems::get_loginitems,
    macho::get_macho,
    plist::get_plist,
    safari::{
        get_safari_downloads, get_safari_history, get_safari_users_downloads,
        get_safari_users_history,
    },
    unifiedlogs::get_unified_log,
};
use crate::runtime::{
    applications::extensions::app_functions, filesystem::extensions::fs_runtime,
    system::extensions::system_functions, unix::extensions::unix_functions,
};
use deno_core::Extension;

/// Include all the `Artemis` function in the `Runtime`
pub(crate) fn setup_extensions() -> Vec<Extension> {
    let extensions = Extension::builder("artemis").ops(grab_functions()).build();
    vec![extensions]
}

/// Link Rust functions to `Deno core`
fn grab_functions() -> Vec<deno_core::OpDecl> {
    let mut exts = vec![
        get_launchd_daemons::decl(),
        get_launchd_agents::decl(),
        get_unified_log::decl(),
        get_plist::decl(),
        get_fsevents::decl(),
        get_macho::decl(),
        get_loginitems::decl(),
        get_users::decl(),
        get_groups::decl(),
        get_emond::decl(),
        get_safari_users_history::decl(),
        get_safari_history::decl(),
        get_safari_users_downloads::decl(),
        get_safari_downloads::decl(),
        get_execpolicy::decl(),
    ];

    exts.append(&mut app_functions());
    exts.append(&mut unix_functions());
    exts.append(&mut system_functions());

    exts.append(&mut fs_runtime());
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
