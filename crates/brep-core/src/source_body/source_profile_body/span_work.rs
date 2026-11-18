//! Shared work accounting for original carrier and pcurve evaluation.
use nurbs_core::{curve::Curve, interval_eval::Interval};

/// Inclusive upper bound on any caller-supplied cell or span budget.
pub(super) const MAX_WORK: usize = 100_000;

/// Number of nonempty knot spans of `curve` touched by the parameter
/// enclosure `t`; each one costs a unit of span work when evaluated.
pub(super) fn touched_spans(curve: &Curve, t: Interval) -> usize {
    (curve.degree..curve.control_points.len())
        .filter(|&i| {
            curve.knots[i] < curve.knots[i + 1]
                && curve.knots[i] <= t.hi
                && curve.knots[i + 1] >= t.lo
        })
        .count()
}
