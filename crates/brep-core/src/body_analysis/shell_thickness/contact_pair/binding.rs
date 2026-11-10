//! Actual coefficient ownership, not a tolerance/endpoint sewing receipt.
use super::refused;
use crate::Result;
use nurbs_core::{
    curve::Curve,
    surface::{Axis, Surface},
    surface_edit,
    surface_offset::regions::Pair,
};
fn along(side: usize) -> Axis {
    if side == 0 || side == 2 {
        Axis::U
    } else {
        Axis::V
    }
}
fn degree(s: &Surface, a: Axis) -> usize {
    match a {
        Axis::U => s.degree_u,
        Axis::V => s.degree_v,
    }
}
pub(super) fn bind(caps: &mut [Surface; 2], regions: &Pair, curve: &Curve) -> Result<Curve> {
    let sides = regions.strips().each_ref().map(|s| s.contact_boundary());
    let common = curve
        .degree
        .max(degree(&caps[0], along(sides[0])))
        .max(degree(&caps[1], along(sides[1])));
    if common > 16 {
        return Err(refused(
            "Shared contact boundary degree exceeds the native cap allowance",
        ));
    }
    let curve = curve.elevate(common)?;
    for i in 0..2 {
        caps[i] = surface_edit::elevate(&caps[i], along(sides[i]), common)?;
        let n = curve.control_points.len();
        let side = sides[i];
        let (p, knots, count) = match along(side) {
            Axis::U => (
                caps[i].degree_u,
                &caps[i].knots_u,
                caps[i].control_points.len(),
            ),
            Axis::V => (
                caps[i].degree_v,
                &caps[i].knots_v,
                caps[i].control_points[0].len(),
            ),
        };
        if p != curve.degree || knots != &curve.knots || count != n {
            return Err(refused(
                "Actual cap/contact boundary clocks or bases are incompatible",
            ));
        }
        for k in 0..n {
            let (u, v) = match side {
                0 => (k, 0),
                1 => (caps[i].control_points.len() - 1, k),
                2 => (k, caps[i].control_points[0].len() - 1),
                _ => (0, k),
            };
            let source = if regions.strips()[i].clock_reversed() {
                n - 1 - k
            } else {
                k
            };
            caps[i].control_points[u][v] = curve.control_points[source].clone();
            caps[i].weights[u][v] = curve.weights[source];
        }
    }
    // Geometry and any elevation rounding MUST be requalified by the caller.
    Ok(curve)
}
