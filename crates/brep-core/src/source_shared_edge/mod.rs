//! Exact canonical world ownership for original source edges with qualified restrictions.
use crate::source_boundary_fragment::{Endpoint, Fragment, Role};
use cad_predicates::BezierIdentity;
use nurbs_core::{Error, Result, curve::Curve, curve_surface_agreement};

mod recipe;
mod same_linear_parameter;
#[path = "../source_shared_edge_sample.rs"]
pub(crate) mod sample;
mod shared_edge;
use recipe::Recipe;
pub(crate) use recipe::Recipe as OriginalRecipe;
pub use sample::{BoundaryChainPoint, BoundaryChainReport, BoundarySample, BoundarySampleReport};
#[path = "../source_shared_edge_endpoint.rs"]
pub(crate) mod endpoint;
pub use endpoint::{BoundaryEndpoint, BoundaryEndpointReport};
#[path = "../source_shared_edge_mesh.rs"]
pub(crate) mod mesh;
pub use mesh::{
    BoundaryMeshLimits, BoundaryMeshPending, BoundaryMeshReport, BoundaryMeshSegment,
    BoundaryMeshVertex,
};
#[cfg(test)]
mod tests;
use same_linear_parameter::*;
pub use shared_edge::*;
