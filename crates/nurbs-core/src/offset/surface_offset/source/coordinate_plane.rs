//! Exact coordinate-plane normal identity for nonlinear original charts.
use crate::{distance_bounds::Interval as I, surface::Surface};
/// Exact rational control-net identity, not a near-planarity tolerance. Positive
/// weights keep the source on this literal coordinate plane. A strictly signed
/// projected Jacobian then proves the exact unit normal, even when the planar
/// source parameterization is nonlinear. Folding/singular boxes do not qualify.
pub(super) fn coordinate_plane(s: &Surface, raw: [I; 3]) -> Option<(usize, f64, f64)> {
    if !s.weights.iter().flatten().all(|w| *w > 0.) {
        return None;
    }
    for axis in 0..3 {
        let c = s.control_points[0][0][axis];
        if s.control_points.iter().flatten().all(|p| p[axis] == c) {
            if raw[axis].lo > 0. {
                return Some((axis, 1., c));
            }
            if raw[axis].hi < 0. {
                return Some((axis, -1., c));
            }
        }
    }
    None
}
