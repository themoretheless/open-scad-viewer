//! Tolerant Boolean fallback: numerical surface/surface intersection with a
//! stated tolerance, for operand pairs the exact analytic matrix refuses.
//!
//! Every face pair whose control hulls overlap is traced: seeds on the first
//! face are projected onto the second by Gauss-Newton, each converged seed
//! that lies inside both trimmed regions starts a predictor/corrector march
//! along `N_a × N_b` in both directions until the curve leaves a trimmed
//! region (the exit is then refined to the crossing of the boundary edge
//! with the other surface, which becomes a shared hit vertex) or closes on
//! itself. Chains are interpolated by cubic B-splines with one common
//! parameterization for the 3D curve and both pcurves; the achieved
//! deviation, measured against both surfaces between the samples, becomes
//! the result's `tolerance_mm` (refused when it would exceed the kernel's
//! 1e-2 mm ceiling). Regions of every face are classified by ray parity
//! against the other solid's exact surfaces and assembled through
//! `imprint_assembly`.
//!
//! This path never claims exactness: the result carries its tolerance, and
//! the caller can tell it apart from an exact result by that field.
use crate::imprint_assembly::{Assembler, ChartVertices, EKey, VKey, face_reversed};
use crate::imprint_pipeline::{self, SpatialRelation};
use crate::uv_regions::{Piece, pcurve_point};
use crate::{Coedge, Model};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};

/// Target chordal tolerance of the traced curves.
const TARGET: f64 = 1e-4;
/// Kernel ceiling for `tolerance_mm`.
const CEILING: f64 = 1e-2;
/// Smallest `tolerance_mm` a tolerant result reports; exact results stay at
/// the operands' 1e-7 mm, so anything above 1e-6 mm is tolerant.
pub const TOLERANT_FLOOR: f64 = 2e-6;
/// Newton convergence for surface/surface points, relative to the scale.
const CONVERGE: f64 = 1e-11;
/// Largest turning angle between consecutive march directions.
const MAX_TURN: f64 = 0.12;
const MAX_POINTS: usize = 4000;
const DENSE: usize = 32;

mod boolean;
#[cfg(test)]
mod tests;
mod tracer;
mod unsupported;
pub use boolean::*;
pub(crate) use tracer::*;
use unsupported::*;
