mod error;
#[cfg(target_os = "linux")]
mod executable;
#[cfg(target_os = "macos")]
mod macho;
#[cfg(target_os = "windows")]
mod pe;

pub(crate) mod artifact;
pub mod process;
