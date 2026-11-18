//! Exact Boolean engine for a canonical sphere against a faceted analytic
//! "mate" solid whose boundary meets the sphere along circles: the box
//! (every face plane cuts a circle) and the axial cylinder (cap planes cut
//! circles, the wall meets the sphere in horizontal rings).
//!
//! The mate contributes the curve network: per face, exact section circles
//! with the phi intervals that lie inside the face, bounded by "hit" vertices
//! where the sphere meets the mate's edges (shared by the two faces at that
//! edge). The engine splits every arc where it crosses a sphere patch seam
//! and wherever its sweep exceeds a quadrant, so each piece is a single-span
//! rational quadratic inside one mate face and one sphere patch; it then
//! reads face and patch regions off small planar UV arrangements (boundary
//! pieces plus arc pcurves), classifies each region against the other
//! operand by an interior sample, trims every kept face's surface to its
//! region, and assembles through `imprint_pipeline`.
//!
//! Every degenerate configuration is an explicit `BREP_UNSUPPORTED_OPERATION`,
//! never a numerical fallback: tangencies, a section through a sphere patch
//! pole or vertex, a hit or seam event that cannot be certified apart from
//! its neighbours, arcs meeting tangentially in UV, and any region sample
//! that falls inside the error band of a classification.
use crate::Model;
pub(crate) use crate::imprint_assembly::face_reversed;
use crate::imprint_assembly::{Assembler, ChartVertices, EKey, VKey};
use crate::imprint_pipeline::{self, SpatialRelation};
use crate::intersections::sphere_sphere::{self, CanonicalSphere, PatchUvSection};
use crate::uv_regions::{Piece, normalized, pcurve_point};
use nurbs_core::{Error, Result, curve::Curve, surface::Surface};
use std::collections::BTreeMap;

pub(crate) const TAU: f64 = std::f64::consts::TAU;
/// UV clearance demanded between an event and a patch corner.
pub(crate) const CORNER: f64 = 1e-7;
/// UV boundary-membership margin.
pub(crate) const BOUNDARY_UV: f64 = 1e-9;
/// Largest 3D sweep of one arc piece (single-span exactness margin).
const MAX_SPAN: f64 = std::f64::consts::FRAC_PI_2 + 1e-9;
/// Angular clearance between two events on one section circle.
pub(crate) const EVENT_GAP: f64 = 1e-8;

mod imprint;
#[cfg(test)]
mod parameter_tests;
mod sphere_inside;
mod unsupported;
pub(crate) use sphere_inside::*;
pub(crate) use unsupported::*;
