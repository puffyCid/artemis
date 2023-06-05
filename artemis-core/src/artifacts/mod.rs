pub(crate) mod applications;
pub(crate) mod os;

#[cfg(target_os = "macos")]
pub(crate) mod macos_collection;

#[cfg(target_os = "windows")]
pub(crate) mod windows_collection;

#[cfg(target_os = "linux")]
pub(crate) mod linux_collection;
