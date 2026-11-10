//! Planar curve offset, offset wires, chord arrangement and surface offset.
//!
//! Groups the former flat modules `curve_offset`, `curve_offset_join`,
//! `curve_offset_wire`, `surface_offset`, `curve_offset_diagnostics` and the
//! seven `chord_*` modules (now `chord::{arrangement, embedding, faces,
//! fill_selection, intersection, winding, witness}`). Old paths keep working
//! through re-export shims in `lib.rs`.
use crate::{Result, distance_bounds::Interval};

pub mod chord;
/// Offset chord-chain diagnostics (formerly `crate::curve_offset_diagnostics`;
/// module directory `offset/diagnostics/`).
pub(crate) mod diagnostics;
pub mod curve_offset;
pub mod curve_offset_join;
pub mod curve_offset_wire;
pub mod surface_offset;
pub mod freeform;
pub mod freeform_blend;
pub mod profile_corner;
/// Surface-pair fillet surfaces: rolling-ball, variable-radius, chordal,
/// G2 and hold-line fillets built on offset-surface intersection marching.
pub mod fillet_surfaces;

/// Offset contact, envelope and trim modules (formerly the flat `offset_*` modules;
/// `lib.rs` keeps the old paths as re-exports).
pub mod source_boundary;
pub mod contact_tangent;
pub mod envelope;
pub mod envelope_fit;
pub mod contact_pcurve;
pub mod contact_trims;
pub mod face_loops;
pub mod patch_boundary;
pub mod contact_predictor;
pub mod contact_path;
pub mod path_trims;
pub mod path_pcurves;

/// Shared 2D interval cross product (reorg DRY #3): replaces the identical
/// private copies that lived in `chord_intersection.rs` and
/// `curve_offset_join.rs` before the grouping.
pub(crate) fn cross2d(a: [Interval; 2], b: [Interval; 2]) -> Result<Interval> {
    a[0].mul(b[1])?.sub(a[1].mul(b[0])?)
}

/// Clearance-qualified outside contour machining paths.
pub mod machining;
/// Formerly the flat `crate::trimmed_offset_contact` module; `lib.rs` keeps that path as a re-export.
pub mod trimmed_contact;
