//! Involute gears as trimmed NURBS solids: spur, helical and herringbone,
//! external with an optional bore or internal (ring) with a rim.
//!
//! The tooth outline is authored per tooth as exact rational arcs (tip and
//! root circles), radial lines below the base circle, and involute flanks
//! fitted by cubic B-spline interpolation of the analytic involute until the
//! fit deviates by less than `FLANK_FIT` from the true curve. Each outline
//! curve becomes one wall face: a ruled surface for a spur gear, or for a
//! helical gear a cubic loft through rotated copies of the curve, refined
//! until it deviates from the exact helical sweep by less than `SWEEP_FIT`.
//! A herringbone gear is two mirrored helical halves sharing the mid-plane
//! outline. Caps are planar faces bounded by the outline and its rotated
//! image, with affine pcurves. All edges are iso-curves of the wall
//! surfaces, so the body is self-consistent to the kernel tolerance; the
//! stated fit bounds describe how far the surfaces sit from the ideal
//! involute helicoid, which no polynomial surface represents exactly.
use crate::tolerant_boolean::interpolate;
use crate::{Body, Coedge, Edge, Face, FaceUse, Loop, Model, Shell, TopologyIds, Vertex};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::f64::consts::{PI, TAU};

/// Largest deviation of a fitted involute flank from the analytic curve.
const FLANK_FIT: f64 = 1e-6;
/// Largest deviation of a lofted helical wall from the exact helical sweep.
const SWEEP_FIT: f64 = 1e-4;
const ARC_WEIGHT_MAX_SWEEP: f64 = PI - 1e-6;

mod gear_spec;
mod gear_with_report;
#[cfg(test)]
mod tests;
pub use gear_spec::*;
pub use gear_with_report::*;
