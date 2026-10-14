//! Certified bounded general NURBS surface/surface intersection.
//!
//! Isolates complete 4D parameter solution strata with outward-rounded
//! Bernstein/interval hull exclusion, Krawczyk uniqueness on terminals,
//! subdivision, and predictor-corrector continuation. Reports carry
//! ToleranceContext evidence, CoedgeTrim maps, BranchGraph components, and
//! revoke topology authority whenever unresolved/resource/conditioning bands
//! remain. Does not use the graph-patch iso fixture.
use crate::intersection::{
    MAX_BOXES, MAX_SPANS, admit_surface, coedge_trim_report as coedge_trim, context, cross3, distance, dot3,
    enclosure_of, homogeneous_grid, hull_diagonal, next_down, next_up, norm3, point3, split_grid_u,
    split_grid_v, surface_spans, tolerance_evidence,
};
use crate::{Result, check, resource, surface::Surface};
use cad_predicates::ToleranceContext;
use value_codec::{Value, json};

const VERSION: &str = "nurbs-ss/1";
const TRANSVERSE_SINE: f64 = 1e-8;
const MAX_CONTINUATION: usize = 512;
const MAX_BRANCHES: usize = 256;

mod grids_excluded_ss;
mod refine_seed;
mod intersect_surface_surface;
use grids_excluded_ss::*;
use refine_seed::*;
pub use intersect_surface_surface::*;

use crate::intersection::{ContactClass,UnresolvedReason};
use crate::foundation::guards::{Budget,require_finite_point};
mod planar; use planar::{PlanarIntersection,plane_plane_line,surface_plane_iso_components};
pub use planar::{ExactBranch,ExactEndpointLocation,OverlapRegion};
mod samples; pub use samples::ContinuationSample;
mod unresolved; pub use unresolved::{UnresolvedClassification,UnresolvedSurfaceIntersection};
mod continuation; use continuation::continue_branch; pub use continuation::{ContinuedBranch,EndpointLocation};
mod branches; pub use branches::Summary as SurfaceIntersectionSummary;
mod junctions;
mod report; use report::mark_junctions; pub use report::{SurfaceSurfaceComponent,SurfaceSurfaceIntersection,TangencyBranch};
#[cfg(feature="codec")] mod serialization;
#[cfg(feature="codec")] pub use serialization::{intersect_surface_surface,ss_resource_probe,verify_ss_coverage};
