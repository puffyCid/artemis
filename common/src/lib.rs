pub mod applications;
pub mod files;
pub mod server;
pub mod system;

#[cfg(target_os = "linux")]
pub mod linux;
#[cfg(target_os = "macos")]
pub mod macos;
#[cfg(target_family = "unix")]
pub mod unix;
#[cfg(target_os = "windows")]
pub mod windows;
