//! Source-expression contour clipping proposal. Exact joins are separate from
//! region containment, face immersion and closed-solid admission.
use crate::{
    source_boundary_fragment::{Endpoint, Fragment, Role},
    trimmed_face_recipe::{self, Boundary, Limits, SourcePointSearch},
};
use cad_predicates::ToleranceContext;
use nurbs_core::{curve::Curve, surface::Surface, Error, Result};

mod proposal;
mod qualify_linear_region;
#[cfg(test)]
mod tests;
pub use proposal::*;
pub use qualify_linear_region::*;
