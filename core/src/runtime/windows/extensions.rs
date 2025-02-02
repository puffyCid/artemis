use super::{
    accounts::{get_alt_users_windows, get_users_windows},
    amcache::{get_alt_amcache, get_amcache},
    bits::{get_bits, get_bits_path},
    ese::{filter_page_data, get_catalog, get_pages, get_table_columns, page_data},
    eventlogs::get_eventlogs,
    jumplists::{get_jumplist_file, get_jumplists},
    ntfs::{read_ads_data, read_raw_file},
    outlook::{get_root_folder, read_attachment, read_folder, read_messages},
    pe::get_pe,
    prefetch::{get_prefetch, get_prefetch_path},
    recyclebin::{get_recycle_bin, get_recycle_bin_file},
    registry::{get_registry, get_sk_info},
    search::get_search,
    services::{get_service_file, get_services},
    shellbags::{get_alt_shellbags, get_shellbags},
    shellitems::js_get_shellitem,
    shimcache::{get_alt_shimcache, get_shimcache},
    shimdb::{get_custom_shimdb, get_shimdb},
    shortcuts::get_lnk_file,
    srum::get_srum,
    tasks::{get_task_file, get_tasks},
    userassist::{get_alt_userassist, get_userassist},
    usnjrnl::{get_alt_usnjrnl, get_alt_usnjrnl_path, get_usnjrnl},
    wmi::get_wmipersist,
};
use deno_core::Extension;

/// Include all the `Artemis` function in the `Runtime`
pub(crate) fn setup_windows_extensions() -> Vec<Extension> {
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
        get_alt_shimcache(),
        get_shimcache(),
        get_registry(),
        get_eventlogs(),
        get_lnk_file(),
        get_usnjrnl(),
        get_alt_usnjrnl(),
        get_alt_usnjrnl_path(),
        get_shellbags(),
        get_alt_shellbags(),
        read_raw_file(),
        read_ads_data(),
        get_pe(),
        get_prefetch(),
        get_prefetch_path(),
        get_userassist(),
        get_alt_userassist(),
        get_amcache(),
        get_alt_amcache(),
        get_shimdb(),
        get_custom_shimdb(),
        get_bits(),
        get_bits_path(),
        get_srum(),
        get_users_windows(),
        get_alt_users_windows(),
        get_search(),
        get_tasks(),
        get_task_file(),
        get_services(),
        get_service_file(),
        get_jumplists(),
        get_jumplist_file(),
        get_recycle_bin(),
        get_recycle_bin_file(),
        get_sk_info(),
        get_catalog(),
        get_pages(),
        page_data(),
        filter_page_data(),
        get_table_columns(),
        js_get_shellitem(),
        get_wmipersist(),
        get_root_folder(),
        read_folder(),
        read_messages(),
        read_attachment(),
    ];

    exts
}

#[cfg(test)]
mod tests {
    use super::{grab_functions, setup_windows_extensions};

    #[test]
    fn test_grab_functions() {
        let results = grab_functions();
        assert!(results.len() > 2)
    }

    #[test]
    fn test_setup_windows_extensions() {
        let results = setup_windows_extensions();
        assert_eq!(results.len(), 1)
    }
}
