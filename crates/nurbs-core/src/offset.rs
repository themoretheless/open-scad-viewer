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
/// Surface-pair fillet surfaces: rolling-ball, variable-radius, chordal,
/// G2 and hold-line fillets built on offset-surface intersection marching.
pub mod fillet_surfaces;

/// Shared 2D interval cross product (reorg DRY #3): replaces the identical
/// private copies that lived in `chord_intersection.rs` and
/// `curve_offset_join.rs` before the grouping.
pub(crate) fn cross2d(a: [Interval; 2], b: [Interval; 2]) -> Result<Interval> {
    a[0].mul(b[1])?.sub(a[1].mul(b[0])?)
}
