//! Crate-private mechanical helpers shared by the audit/verification family
//! (`sweeps/audit/*`, `trim/trim_region_audit`, `join/periodic_*_verification`,
//! `analysis/curve_regularity`, `analysis/surface_injectivity`).
//!
//! Scope decision (DRY #7): the family shares no report types, no reason
//! vocabulary and no `is_clean`/`len` API — every audit keeps its own
//! contract. What repeats byte-for-byte in 3+ modules is knot-domain
//! enumeration: nonempty-span filtering and natural-domain extraction.
//! Only that machinery lives here; audit semantics stay per-module.
use crate::{curve::Curve, surface::Surface};

/// Indices `i` in `degree..count` with `knots[i] < knots[i + 1]`, i.e. the
/// nonempty knot spans. Fully repeated (empty) spans carry no interior and
/// are skipped identically by every audit traversal and budget loop.
pub(crate) fn nonempty_spans(knots: &[f64], degree: usize, count: usize) -> Vec<usize> {
    (degree..count).filter(|&i| knots[i] < knots[i + 1]).collect()
}

/// Natural parameter domain `[knots[degree], knots[count]]` of a basis with
/// `count` control points. This is the represented interval regardless of
/// clamping; clamping is a separate audit obligation.
pub(crate) fn natural_domain(knots: &[f64], degree: usize, count: usize) -> [f64; 2] {
    [knots[degree], knots[count]]
}

/// Natural parameter domains of both tensor axes, in `[u, v]` order.
pub(crate) fn surface_domains(s: &Surface) -> [[f64; 2]; 2] {
    [
        natural_domain(&s.knots_u, s.degree_u, s.control_points.len()),
        natural_domain(&s.knots_v, s.degree_v, s.control_points[0].len()),
    ]
}

/// Nonempty curve spans as `(span index, [lo, hi])` pairs, in knot order.
pub(crate) fn curve_span_domains(c: &Curve) -> Vec<(usize, [f64; 2])> {
    nonempty_spans(&c.knots, c.degree, c.control_points.len())
        .into_iter()
        .map(|i| (i, [c.knots[i], c.knots[i + 1]]))
        .collect()
}
