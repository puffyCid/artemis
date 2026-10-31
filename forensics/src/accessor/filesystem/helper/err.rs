use crate::accessor::error::AccessorError;
use std::io;

/// Handle `AccessorError` errors to `io::Error`
pub(crate) fn accessor_to_io(err: AccessorError) -> io::Error {
    io::Error::other(err.to_string())
}
