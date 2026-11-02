//! Independent document/operation design informed by architectural observations.
//! Geometry is implemented by the frozen, independently authored clean package.
//! No target code, ABI, proprietary library, or target data format is included.
pub mod document;
pub mod operation;
pub use document::{
    Cancellation, Computed, Display, Document, Entity, EntityId, Geometry, GeometryKind, Job,
    PublishError, Receipt, Resolution, Snapshot, Transaction,
};
pub use modeling_clean as geometry;
pub use operation::Operation;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Error {
    Cancelled,
    Geometry(modeling_clean::Error),
    MissingEntity(EntityId),
    WrongGeometry {
        id: EntityId,
        expected: &'static str,
    },
    StaleRevision {
        expected: u64,
        actual: u64,
    },
    WrongDocument,
    FailedTransaction,
    Limit(&'static str),
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
