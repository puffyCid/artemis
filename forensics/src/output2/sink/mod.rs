mod aws;
pub(crate) mod factory;
#[cfg(feature = "gcp")]
mod gcp;
mod local;
mod output_handle;
mod output_sink;
