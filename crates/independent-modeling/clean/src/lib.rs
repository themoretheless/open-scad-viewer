//! Independent geometry definitions and explicitly faceted modeling operations.
//! No host CAD dependencies, application code, FFI, filesystem access or runtime.
pub mod boolean;
pub mod curve;
pub mod math;
pub mod mesh;
pub mod profile;
pub mod surface;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    InvalidInput(&'static str),
    Unsupported(&'static str),
    Degenerate(&'static str),
    Limit(&'static str),
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;

pub(crate) fn require(ok: bool, message: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::InvalidInput(message))
    }
}
