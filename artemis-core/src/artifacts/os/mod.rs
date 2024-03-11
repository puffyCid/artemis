pub(crate) mod files;
pub(crate) mod processes;
pub(crate) mod systeminfo;

#[cfg(target_family = "unix")]
pub(crate) mod macos;

#[cfg(target_family = "unix")]
pub(crate) mod unix;

pub(crate) mod windows;

#[cfg(target_family = "unix")]
pub(crate) mod linux;
