use super::{
    accounts::{get_alt_users, get_users},
    amcache::{get_alt_amcache, get_amcache},
    bits::{get_bits, get_bits_path},
    eventlogs::get_eventlogs,
    ntfs::{read_ads_data, read_raw_file},
    pe::get_pe,
    prefetch::{get_alt_prefetch, get_prefetch, get_prefetch_path},
    processes::get_processes,
    registry::get_registry,
    search::get_search,
    shellbags::{get_alt_shellbags, get_shellbags},
    shimcache::{get_alt_shimcache, get_shimcache},
    shimdb::{get_alt_shimdb, get_custom_shimdb, get_shimdb},
    shortcuts::get_lnk_file,
    srum::get_srum,
    systeminfo::get_systeminfo,
    userassist::{get_alt_userassist, get_userassist},
    usnjrnl::{get_alt_usnjrnl, get_usnjrnl},
};
use crate::runtime::{
    applications::extensions::app_functions, system::extensions::system_functions,
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
        get_alt_shimcache::decl(),
        get_shimcache::decl(),
        get_registry::decl(),
        get_eventlogs::decl(),
        get_lnk_file::decl(),
        get_usnjrnl::decl(),
        get_alt_usnjrnl::decl(),
        get_shellbags::decl(),
        get_alt_shellbags::decl(),
        read_raw_file::decl(),
        read_ads_data::decl(),
        get_pe::decl(),
        get_prefetch::decl(),
        get_alt_prefetch::decl(),
        get_prefetch_path::decl(),
        get_userassist::decl(),
        get_alt_userassist::decl(),
        get_amcache::decl(),
        get_alt_amcache::decl(),
        get_shimdb::decl(),
        get_alt_shimdb::decl(),
        get_custom_shimdb::decl(),
        get_bits::decl(),
        get_bits_path::decl(),
        get_srum::decl(),
        get_users::decl(),
        get_alt_users::decl(),
        get_search::decl(),
    ];
    exts.append(&mut app_functions());
    exts.append(&mut system_functions());
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
