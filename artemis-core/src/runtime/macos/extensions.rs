use super::{
    accounts::{get_groups_macos, get_users_macos},
    emond::get_emond,
    execpolicy::get_execpolicy,
    fsevents::get_fsevents,
    launchd::{get_launchd_agents, get_launchd_daemons},
    loginitems::get_loginitems,
    macho::get_macho,
    plist::{get_plist, get_plist_data},
    safari::{
        get_safari_downloads, get_safari_history, get_safari_users_downloads,
        get_safari_users_history,
    },
    spotlight::{get_spotlight, setup_spotlight_parser},
    sudo::get_sudologs_macos,
    unifiedlogs::{get_unified_log, setup_unified_log_parser},
};
use crate::runtime::{
    applications::extensions::app_functions, encoding::extensions::enocoding_runtime,
    environment::extensions::env_runtime, filesystem::extensions::fs_runtime,
    http::extensions::http_functions, nom::extensions::nom_functions,
    system::extensions::system_functions, time::extensions::time_functions,
    unix::extensions::unix_functions,
};
use deno_core::{Extension, Op};

/// Include all the `Artemis` function in the `Runtime`
pub(crate) fn setup_macos_extensions() -> Vec<Extension> {
    let extensions = Extension {
        name: "artemis",
        ops: grab_functions().into(),
        ..Default::default()
    };
    vec![extensions]
}

/// Link Rust functions to `Deno core`
fn grab_functions() -> Vec<deno_core::OpDecl> {
    let mut exts = vec![
        get_launchd_daemons::DECL,
        get_launchd_agents::DECL,
        get_unified_log::DECL,
        setup_unified_log_parser::DECL,
        get_plist::DECL,
        get_plist_data::DECL,
        get_fsevents::DECL,
        get_macho::DECL,
        get_loginitems::DECL,
        get_users_macos::DECL,
        get_groups_macos::DECL,
        get_emond::DECL,
        get_safari_users_history::DECL,
        get_safari_history::DECL,
        get_safari_users_downloads::DECL,
        get_safari_downloads::DECL,
        get_execpolicy::DECL,
        get_sudologs_macos::DECL,
        get_spotlight::DECL,
        setup_spotlight_parser::DECL,
    ];

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
    use super::{grab_functions, setup_macos_extensions};

    #[test]
    fn test_grab_functions() {
        let results = grab_functions();
        assert!(results.len() > 2);
    }

    #[test]
    fn test_setup_macos_extensions() {
        let results = setup_macos_extensions();
        assert_eq!(results.len(), 1);
    }
}
