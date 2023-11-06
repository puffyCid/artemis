use super::{
    accounts::{get_alt_users, get_users},
    amcache::{get_alt_amcache, get_amcache},
    bits::{get_bits, get_bits_path},
    eventlogs::get_eventlogs,
    jumplists::{get_alt_jumplists, get_jumplist_file, get_jumplists},
    ntfs::{read_ads_data, read_raw_file},
    pe::get_pe,
    prefetch::{get_alt_prefetch, get_prefetch, get_prefetch_path},
    recyclebin::{get_alt_recycle_bin, get_recycle_bin, get_recycle_bin_file},
    registry::get_registry,
    search::get_search,
    services::{get_alt_services, get_service_file, get_services},
    shellbags::{get_alt_shellbags, get_shellbags},
    shimcache::{get_alt_shimcache, get_shimcache},
    shimdb::{get_alt_shimdb, get_custom_shimdb, get_shimdb},
    shortcuts::get_lnk_file,
    srum::get_srum,
    tasks::{get_alt_tasks, get_task_file, get_tasks},
    userassist::{get_alt_userassist, get_userassist},
    usnjrnl::{get_alt_usnjrnl, get_usnjrnl},
};
use crate::runtime::{
    applications::extensions::app_functions, encoding::extensions::enocoding_runtime,
    environment::extensions::env_runtime, filesystem::extensions::fs_runtime,
    http::extensions::http_functions, nom::extensions::nom_functions,
    system::extensions::system_functions, time::extensions::time_functions,
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
    let mut exts = vec![
        get_alt_shimcache::DECL,
        get_shimcache::DECL,
        get_registry::DECL,
        get_eventlogs::DECL,
        get_lnk_file::DECL,
        get_usnjrnl::DECL,
        get_alt_usnjrnl::DECL,
        get_shellbags::DECL,
        get_alt_shellbags::DECL,
        read_raw_file::DECL,
        read_ads_data::DECL,
        get_pe::DECL,
        get_prefetch::DECL,
        get_alt_prefetch::DECL,
        get_prefetch_path::DECL,
        get_userassist::DECL,
        get_alt_userassist::DECL,
        get_amcache::DECL,
        get_alt_amcache::DECL,
        get_shimdb::DECL,
        get_alt_shimdb::DECL,
        get_custom_shimdb::DECL,
        get_bits::DECL,
        get_bits_path::DECL,
        get_srum::DECL,
        get_users::DECL,
        get_alt_users::DECL,
        get_search::DECL,
        get_tasks::DECL,
        get_alt_tasks::DECL,
        get_task_file::DECL,
        get_services::DECL,
        get_alt_services::DECL,
        get_service_file::DECL,
        get_jumplists::DECL,
        get_alt_jumplists::DECL,
        get_jumplist_file::DECL,
        get_recycle_bin::DECL,
        get_alt_recycle_bin::DECL,
        get_recycle_bin_file::DECL,
    ];
    exts.append(&mut app_functions());
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
        assert!(results.len() > 2)
    }

    #[test]
    fn test_setup_extensions() {
        let results = setup_extensions();
        assert_eq!(results.len(), 1)
    }
}
