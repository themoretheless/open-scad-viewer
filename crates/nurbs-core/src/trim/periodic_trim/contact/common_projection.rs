//! Exact endpoint germ ordering under an identical longitudinal projection.
use super::ContactPort;
use crate::{curve::Curve, exact_products::sum_sign};
use std::cmp::Ordering;

/// Sufficient local order only: two clamped rational B-spline endpoint spans
/// must have exactly proportional oriented knot gaps and weights, and exactly
/// identical projection of their active controls onto the leading
/// ray. Their common longitudinal coordinate is strictly monotone sufficiently
/// near the endpoint, since its first nonzero coefficient is positive. Hence
/// normal order at equal parameter is also order at equal longitudinal distance.
/// No sampled radius, truncated root estimate or rounded vector is used.
pub(super) fn compare(
    a: &ContactPort,
    b: &ContactPort,
    ca: &Curve,
    cb: &Curve,
    origin: [f64; 2],
) -> Option<Ordering> {
    let p = ca.degree;
    if p != cb.degree {
        return None;
    }
    let index = |c: &Curve, incoming: bool, i: usize| {
        if incoming {
            c.control_points.len() - 1 - i
        } else {
            i
        }
    };
    let knot = |c: &Curve, incoming: bool, j: usize| {
        if incoming {
            c.knots[c.control_points.len() - j]
        } else {
            c.knots[p + j]
        }
    };
    let a0 = knot(ca, a.incoming, 0);
    let b0 = knot(cb, b.incoming, 0);
    let a1 = knot(ca, a.incoming, 1);
    let b1 = knot(cb, b.incoming, 1);
    // Equal normalized first-span basis, without computing either knot gap
    // or their ratio in binary64. Reversal changes the sign of both gaps.
    for j in 2..=p {
        let aj = knot(ca, a.incoming, j);
        let bj = knot(cb, b.incoming, j);
        if sum_sign(&[
            (aj, b1),
            (-aj, b0),
            (-a0, b1),
            (a0, b0),
            (-bj, a1),
            (bj, a0),
            (b0, a1),
            (-b0, a0),
        ]) != Ordering::Equal
        {
            return None;
        }
    }
    let wa0 = ca.weights[index(ca, a.incoming, 0)];
    let wb0 = cb.weights[index(cb, b.incoming, 0)];
    let ray = a.outward_control;
    let dot_difference = |x: &[f64], y: &[f64]| {
        sum_sign(&[
            (x[0], ray[0]),
            (-x[0], origin[0]),
            (-y[0], ray[0]),
            (y[0], origin[0]),
            (x[1], ray[1]),
            (-x[1], origin[1]),
            (-y[1], ray[1]),
            (y[1], origin[1]),
        ])
    };
    for i in 0..=p {
        let ia = index(ca, a.incoming, i);
        let ib = index(cb, b.incoming, i);
        if sum_sign(&[(ca.weights[ia], wb0), (-cb.weights[ib], wa0)]) != Ordering::Equal
            || dot_difference(&ca.control_points[ia], &cb.control_points[ib]) != Ordering::Equal
        {
            return None;
        }
    }
    for i in 1..=p {
        let x = &ca.control_points[index(ca, a.incoming, i)];
        let y = &cb.control_points[index(cb, b.incoming, i)];
        // cross(ray-origin, y-x), all eight original binary64 products.
        let cross = sum_sign(&[
            (ray[0], y[1]),
            (-origin[0], y[1]),
            (-ray[0], x[1]),
            (origin[0], x[1]),
            (-ray[1], y[0]),
            (origin[1], y[0]),
            (ray[1], x[0]),
            (-origin[1], x[0]),
        ]);
        if cross != Ordering::Equal {
            return Some(cross.reverse());
        }
    }
    // Identical endpoint spans are not an isolated contact and provide no ordering.
    None
}
