#[derive(PartialEq)]
pub(crate) enum Options {
    Processes,
    Files,
    #[cfg(target_os = "macos")]
    Unifiedlogs,
    #[cfg(target_os = "windows")]
    Shellbags,
    #[cfg(target_os = "windows")]
    Shortcuts,
    #[cfg(target_os = "windows")]
    Bits,
    #[cfg(target_os = "windows")]
    Registry,
    #[cfg(target_os = "windows")]
    Srum,
    #[cfg(target_os = "windows")]
    Eventlogs,
    #[cfg(target_os = "windows")]
    Prefetch,
    #[cfg(target_os = "windows")]
    Shimdb,
    #[cfg(target_os = "windows")]
    Shimcache,
    #[cfg(target_os = "windows")]
    Amcache,
    #[cfg(target_os = "windows")]
    UsnJrnl,
    #[cfg(target_os = "windows")]
    Users,
    #[cfg(target_os = "windows")]
    Search,
    #[cfg(target_os = "windows")]
    Tasks,
    #[cfg(target_os = "windows")]
    Services,
    #[cfg(target_os = "windows")]
    Jumplists,
    #[cfg(target_os = "windows")]
    RecycleBin,
    #[cfg(target_os = "windows")]
    Userassist,
    #[cfg(target_os = "windows")]
    RawFiles,
    None,
}
