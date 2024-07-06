use super::{
    accounts::{get_groups_macos, get_users_macos},
    bookmarks::get_bookmark,
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
    unifiedlogs::get_unified_log,
};
use crate::runtime::{
    applications::extensions::app_functions, compression::extensions::compression_functions,
    decryption::extensions::decryption_functions, encoding::extensions::enocoding_functions,
    environment::extensions::env_runtime, filesystem::extensions::fs_runtime,
    http::extensions::http_functions, nom::extensions::nom_functions,
    system::extensions::system_functions, time::extensions::time_functions,
    unix::extensions::unix_functions,
};
use deno_core::Extension;

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
        get_launchd_daemons(),
        get_launchd_agents(),
        get_unified_log(),
        get_plist(),
        get_plist_data(),
        get_fsevents(),
        get_macho(),
        get_loginitems(),
        get_users_macos(),
        get_groups_macos(),
        get_emond(),
        get_safari_users_history(),
        get_safari_history(),
        get_safari_users_downloads(),
        get_safari_downloads(),
        get_execpolicy(),
        get_sudologs_macos(),
        get_spotlight(),
        setup_spotlight_parser(),
        get_bookmark(),
    ];

    exts.append(&mut app_functions());
    exts.append(&mut unix_functions());
    exts.append(&mut system_functions());

    exts.append(&mut fs_runtime());
    exts.append(&mut env_runtime());
    exts.append(&mut enocoding_functions());

    exts.append(&mut nom_functions());
    exts.append(&mut time_functions());
    exts.append(&mut http_functions());
    exts.append(&mut compression_functions());
    exts.append(&mut decryption_functions());

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
