pub mod applications;
pub mod files;
pub mod server;
pub mod system;

#[cfg(target_family = "unix")]
pub mod linux;
#[cfg(target_family = "unix")]
pub mod macos;
#[cfg(target_family = "unix")]
pub mod unix;
pub mod windows;
