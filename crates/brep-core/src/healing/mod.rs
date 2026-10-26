//! Compound healing pipelines with typed stage evidence.
#[cfg(feature = "codec")]
pub mod edge_fix;
#[cfg(feature = "codec")]
pub mod tolerant_sew;
#[cfg(feature = "codec")]
pub mod surface_extension;
#[cfg(feature = "codec")]
pub mod canonical_cast;

#[cfg(feature = "codec")]
pub use edge_fix::{EdgeFixBudget, fix_edges, split_edge};
#[cfg(feature = "codec")]
pub use tolerant_sew::{SewingBudget, sew_tolerant};
#[cfg(feature = "codec")]
pub use surface_extension::{SurfaceExtensionBudget, extend_surface_seams};
#[cfg(feature = "codec")]
pub use canonical_cast::{CanonicalCastBudget, cast_canonical_surfaces};
