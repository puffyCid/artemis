mod error;
mod options;
pub(crate) mod parser;
mod registry;
mod service;
#[cfg(target_os = "windows")]
pub(crate) mod state;
