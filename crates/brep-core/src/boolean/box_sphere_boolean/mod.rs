//! Box × sphere: the `Mate` of `sphere_mate` for the six-face `cuboid`
//! (bilinear affine faces over the unit square, unit weights, four-coedge
//! unit-square trims, rigid placement admitted). Every crossing face plane
//! cuts the sphere in a circle; the sphere meets each box edge in at most two
//! hit vertices shared by the two faces at that edge, so the sphere may sit
//! over a face, an edge or a corner, or run through a slab or a bar.
use crate::Model;
use crate::imprint_pipeline::SpatialRelation;
use crate::intersections::sphere_sphere::{self, CanonicalSphere};
use crate::sphere_mate::{
    self, BOUNDARY_UV, Circle, Imprint, Mate, PlaneContact, TAU, affine_arc_pcurve, cross, dist,
    dot, face_reversed, norm, plane_contact, point3, sub, unsupported,
};
use nurbs_core::{Result, curve::Curve};

/// Structural recognition band for the box faces, relative to their size.
const RECOGNITION: f64 = 1e-9;

mod box_face;
mod dispatch;
mod mate;
#[cfg(test)]
mod tests;
use box_face::*;
pub(crate) use dispatch::*;
