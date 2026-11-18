//! A root branch covers the entire small gap; a literal midpoint is only a
//! coefficient proposal. Complete modified curve proofs remain mandatory.
use super::{Limits, Span, endpoint_clock, ordered};
use crate::{
    Result, check, distance_bounds::Interval as I, offset_contact_pcurve as uv,
    offset_contact_tangent as tangent, surface::Surface, surface_contact::Witness,
    surface_offset::ContactBand,
};
pub struct Join {
    pub parameters: [[f64; 2]; 2],
    pub point: [f64; 3],
    pub uv_error_upper: [f64; 2],
    pub world_error_upper: f64,
    /// Contains the entire original root transition, not merely sampled ends.
    pub source_root_window: Witness,
    pub endpoint_roots: [Witness; 2],
    /// Following piece's driving coordinate differentiated by preceding clock.
    pub clock_derivative: [f64; 2],
    pub root_queries: usize,
    pub band_queries: usize,
}
fn restore(mut w: Witness, owner: usize) -> Witness {
    if owner == 1 {
        std::mem::swap(&mut w.first_uv, &mut w.second_uv);
    }
    w
}
fn params(w: &Witness) -> [[[f64; 2]; 2]; 2] {
    [w.first_uv, w.second_uv]
}
fn hull(a: [f64; 2], b: [f64; 2]) -> [f64; 2] {
    [a[0].min(b[0]), a[1].max(b[1])]
}
fn midpoint(a: [f64; 2]) -> f64 {
    a[0] * 0.5 + a[1] * 0.5
}
fn error<const N: usize>(point: [f64; N], window: [[f64; 2]; N]) -> Result<f64> {
    let mut r = [I::point(0.); 3];
    for k in 0..N {
        r[k] = I::point(point[k]).sub(I::new(window[k][0], window[k][1])?)?;
    }
    Ok(tangent::speed(r)?.hi)
}
fn section(
    s: [&Surface; 2],
    d: [f64; 2],
    span: Span,
    start: bool,
    l: Limits,
) -> Result<(Witness, usize)> {
    let (s, d) = ordered(s, d, span);
    let (w, n) = uv::section(
        crate::offset::contact_pcurve::OffsetFaces {
            surfaces: s,
            distances: d,
        },
        span.axis,
        endpoint_clock(span, start),
        span.free,
        span.other,
        l.spans,
        l.refinements,
    )?;
    Ok((
        restore(
            w.ok_or_else(|| crate::input("Original chain endpoint root remains unresolved"))?,
            span.owner,
        ),
        n,
    ))
}
pub(super) fn prove(
    sources: [&Surface; 2],
    distances: [f64; 2],
    a: Span,
    b: Span,
    tol_uv: f64,
    tol_mm: f64,
    l: Limits,
) -> Result<Join> {
    let (wa, qa) = section(sources, distances, a, false, l)?;
    let (wb, qb) = section(sources, distances, b, true, l)?;
    let root_b = params(&wb);
    let t = endpoint_clock(a, false);
    let next = root_b[a.owner][a.axis];
    check(
        if a.reversed {
            next[1] <= t
        } else {
            next[0] >= t
        },
        "Chart-switch endpoint ordering remains unresolved or backtracks",
    )?;
    let transition = hull([t; 2], next);
    // The preceding endpoint root is in the original A tube. The following
    // endpoint's complete root enclosure is included by construction. A fresh
    // uniform uniqueness proof then puts BOTH roots on the SAME continuous
    // branch, including all original roots between their A-clock coordinates.
    let free = hull(a.free, root_b[a.owner][1 - a.axis]);
    let other = [
        hull(a.other[0], root_b[1 - a.owner][0]),
        hull(a.other[1], root_b[1 - a.owner][1]),
    ];
    let drive = if transition[0] < transition[1] {
        transition
    } else {
        // A literal equal-clock join has no missing root arc. Use a small
        // original-domain collar only for regularity and clock-orientation;
        // its width is NOT substituted for the zero-gap error enclosure.
        let source = sources[a.owner];
        let domain = if a.axis == 0 {
            [
                source.knots_u[source.degree_u],
                source.knots_u[source.control_points.len()],
            ]
        } else {
            [
                source.knots_v[source.degree_v],
                source.knots_v[source.control_points[0].len()],
            ]
        };
        let w = (a.drive[1] - a.drive[0]) * 1e-6;
        [(t - w).max(domain[0]), (t + w).min(domain[1])]
    };
    check(
        drive[0] < drive[1],
        "Chart-switch proof interval is numerically unrepresentable",
    )?;
    let (s, d) = ordered(sources, distances, a);
    let proof = tangent::certify_refined(
        crate::offset::contact_pcurve::OffsetFaces {
            surfaces: s,
            distances: d,
        },
        a.axis,
        drive,
        free,
        other,
        l.spans,
        l.refinements,
    )?;
    check(
        proof.report.regular,
        "Chart-switch original branch regularity remains unresolved",
    )?;
    let derivative = proof
        .report
        .derivatives
        .ok_or_else(|| crate::input("Missing original clock derivative"))?;
    let clock_derivative = if b.owner == a.owner {
        if b.axis == a.axis {
            [1.; 2]
        } else {
            derivative[0]
        }
    } else {
        derivative[1 + b.axis]
    };
    check(
        if a.reversed == b.reversed {
            clock_derivative[0] > 0.
        } else {
            clock_derivative[1] < 0.
        },
        "Chart switch does not prove consistent traversal orientation",
    )?;
    let window = if transition[0] == transition[1] {
        wa.clone()
    } else {
        let ContactBand::ContinuousBranch(w) = proof.report.contact else {
            return Err(crate::input("Missing uniform transition root authority"));
        };
        restore(w, a.owner)
    };
    let parameters = params(&window).map(|s| s.map(midpoint));
    let point = window.point.map(midpoint);
    let uv_error_upper = [
        error(parameters[0], window.first_uv)?,
        error(parameters[1], window.second_uv)?,
    ];
    let world_error_upper = error(point, window.point)?;
    check(
        uv_error_upper.iter().all(|e| *e <= tol_uv) && world_error_upper <= tol_mm,
        "Whole original chart-switch gap exceeds the chain error tolerances",
    )?;
    Ok(Join {
        parameters,
        point,
        uv_error_upper,
        world_error_upper,
        source_root_window: window,
        endpoint_roots: [wa, wb],
        clock_derivative,
        root_queries: qa + qb,
        band_queries: proof.band_queries,
    })
}
