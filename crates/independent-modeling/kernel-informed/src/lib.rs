//! Geometry-level alternative informed by target behavior and structural evidence.
//! No proprietary code, native runtime, host CAD crate or document layer.
#![doc = include_str!("../README.md")]
pub mod brep;
pub mod loft;
pub mod planar;
pub use modeling_clean::{curve, math, mesh, surface};

#[derive(Clone, Debug, PartialEq)]
pub enum Error {
    Geometry(modeling_clean::Error),
    Invalid(&'static str),
    Unresolved(&'static str),
    Budget(&'static str),
}
impl From<modeling_clean::Error> for Error {
    fn from(e: modeling_clean::Error) -> Self {
        Self::Geometry(e)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}
pub type Result<T> = std::result::Result<T, Error>;
pub(crate) fn check(ok: bool, message: &'static str) -> Result<()> {
    if ok {
        Ok(())
    } else {
        Err(Error::Invalid(message))
    }
}
