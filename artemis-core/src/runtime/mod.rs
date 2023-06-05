mod applications;
pub(crate) mod deno;
mod error;
mod run;

#[cfg(target_family = "unix")]
mod unix;

#[cfg(target_os = "macos")]
mod macos;

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;
