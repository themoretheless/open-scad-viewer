//! Finite analytic planar arrangements over retained rational NURBS spans.
//!
//! Lines and circular quadratic arcs are admitted. Intersections, winding and
//! areas use their analytic carriers; output boundaries are trims/reversals of
//! the input curves, never chords or fitted replacements. Ill-conditioned or
//! tolerance-ambiguous events fail explicitly. This is numerical binary64
//! analytic geometry, not an independent exact-arithmetic certificate.
use nurbs_core::{Error, Result, curve::Curve};
use std::f64::consts::{PI, TAU};

const MAX_SPANS: usize = 256;
const MAX_FRAGMENTS: usize = 2048;
type Point = [f64; 2];

#[cfg(test)]
#[path = "../tests/planar_trim.rs"]
mod tests;

mod locate_point;
mod point_location;
mod validate;
pub use locate_point::*;
pub use point_location::*;
pub use validate::*;

pub fn span_green_area(curve: &Curve, origin: [f64; 2], tolerance: f64) -> Result<f64> {
    curve.validate()?;
    if curve.control_points.iter().any(|p| p.len() != 2) {
        return Err(invalid("Planar trims must be 2D curves"));
    }
    let d = curve.domain();
    let mut knots: Vec<_> = curve
        .knots
        .iter()
        .copied()
        .filter(|v| *v >= d[0] && *v <= d[1])
        .collect();
    knots.dedup();
    if knots.len() > MAX_SPANS + 1 {
        return Err(limit());
    }
    knots.windows(2).try_fold(0., |sum, p| {
        let piece = curve.trim(p[0], p[1])?;
        Ok(sum + Span::from_curve(piece, 0, tolerance, origin)?.area([0., 0.]))
    })
}
