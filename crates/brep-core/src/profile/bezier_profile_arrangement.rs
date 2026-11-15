//! Arrangement steps shared by the profile Boolean and normalization: carrier
//! alias classes, retained-fragment agreement and the common work budgets.
use super::{Result, refused};
use crate::Curve;

/// Source spans admitted into one arrangement.
pub(super) const MAX_ARRANGEMENT_SPANS: usize = 1022;
/// Candidate pairs that reach an intersection query.
pub(super) const PAIR_BUDGET: usize = 8192;
/// Retained fragments, before and after assembly.
pub(super) const FRAGMENT_BUDGET: usize = 2048;
/// Control-hull gap, in tolerances, that rejects a pair without a query.
pub(super) const HULL_GAP: f64 = 8.;
/// Roundoff multiple admitted when two retained fragments are compared.
const AGREEMENT_ROUNDOFF: f64 = 4096.;
/// Fraction of the tolerance that bounds the roundoff agreement limit.
pub(super) const ROUNDOFF_TOLERANCE_FRACTION: f64 = 0.001;

/// First and last element of a control sequence.
pub(super) fn control_ends<T>(points: &[T]) -> Result<(&T, &T)> {
    match (points.first(), points.last()) {
        (Some(first), Some(last)) => Ok((first, last)),
        _ => Err(refused("Bézier span has no control points")),
    }
}

/// Redirect every member of the class of `absorbed` to the class of `keeper`
/// and return the previous `(absorbed, keeper)` class ids.
pub(super) fn merge_alias(aliases: &mut [usize], keeper: usize, absorbed: usize) -> (usize, usize) {
    let old = aliases[absorbed];
    let new = aliases[keeper];
    for alias in aliases.iter_mut() {
        if *alias == old {
            *alias = new;
        }
    }
    (old, new)
}

/// Roundoff bound for comparing two retained fragments.
pub(super) fn agreement_limit(a: &Curve, b: &Curve, tolerance: f64) -> f64 {
    let scale = a
        .control_points
        .iter()
        .chain(&b.control_points)
        .flatten()
        .map(|v| v.abs())
        .fold(1., f64::max);
    (AGREEMENT_ROUNDOFF * f64::EPSILON * scale).min(tolerance * ROUNDOFF_TOLERANCE_FRACTION)
}

/// Exact, nonlinear or roundoff-bounded agreement of two oriented fragments.
pub(super) fn fragments_agree(first: &Curve, second: &Curve, tolerance: f64) -> Result<bool> {
    let limit = agreement_limit(first, second, tolerance);
    Ok(crate::profile_certificate::certify_curve_agreement(
        first,
        second,
        first.domain(),
        second.domain(),
    )
    .is_ok()
        || crate::profile_certificate::certify_nonlinear_curve_agreement(
            first,
            second,
            first.domain(),
            second.domain(),
        )
        .is_ok()
        || nurbs_core::intersection::planar_coincidence::agreement(first, second)?
            .is_some_and(|e| e <= limit))
}
