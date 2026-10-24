//! Exact canonical world ownership for original source edges with qualified restrictions.
use crate::source_boundary_fragment::{Endpoint, Fragment, Role};
use cad_predicates::BezierIdentity;
use nurbs_core::{curve::Curve, curve_surface_agreement, Error, Result};

mod shared_edge;
mod same_linear_parameter;
#[path = "../source_shared_edge_sample.rs"]
pub(crate) mod sample;
pub use sample::{BoundarySample, BoundarySampleReport, BoundaryChainPoint, BoundaryChainReport};
#[path = "../source_shared_edge_endpoint.rs"]
pub(crate) mod endpoint;
pub use endpoint::{BoundaryEndpoint, BoundaryEndpointReport};
#[path = "../source_shared_edge_mesh.rs"]
pub(crate) mod mesh;
pub use mesh::{BoundaryMeshLimits,BoundaryMeshVertex,BoundaryMeshSegment,BoundaryMeshPending,BoundaryMeshReport};
#[cfg(test)]
mod tests;
pub use shared_edge::*;
use same_linear_parameter::*;
