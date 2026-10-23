//! Certified bounded general NURBS surface/surface intersection.
//!
//! Isolates complete 4D parameter solution strata with outward-rounded
//! Bernstein/interval hull exclusion, Krawczyk uniqueness on terminals,
//! subdivision, and predictor-corrector continuation. Reports carry
//! ToleranceContext evidence, CoedgeTrim maps, BranchGraph components, and
//! revoke topology authority whenever unresolved/resource/conditioning bands
//! remain. Does not use the graph-patch iso fixture.
use crate::intersection::{
    MAX_BOXES, MAX_SPANS, admit_surface, coedge_trim, context, cross3, distance, dot3,
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
