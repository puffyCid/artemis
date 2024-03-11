pub(crate) mod applications;
pub(crate) mod os;

#[cfg(target_family = "unix")]
pub(crate) mod macos_collection;
