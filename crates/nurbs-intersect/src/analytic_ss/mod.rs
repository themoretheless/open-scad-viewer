//! Finite analytic surface/surface and curve/plane Complete matrix (G5a/G5c start).
//!
//! Only the frozen contacts below may publish `Coverage::Complete`. Out-of-matrix
//! pairs refuse with typed Incomplete/Unsupported. No mesh, seed marching, or
//! tolerance growth participates.

use crate::{
    Contact, Coverage, CurvePlaneComponent, CurvePoint, Options, Plane, Report, UnresolvedReason,
};
use nurbs_core::{Error, Result};


use crate::vec3::{add, cross, dot, norm, scale as mul, sub};

mod analytic_ss_component;
mod cylinder_sphere;
#[cfg(test)]
mod tests;
pub use analytic_ss_component::*;
pub use cylinder_sphere::*;
