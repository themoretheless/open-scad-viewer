//! Whole-domain error of an ACTUAL saved surface against the original normal
//! offset. Interval original-span jets and a mean-value cover include rounding.
//! Local regularity/focal checks do not prove global embedding or shell closure.
use crate::{Result, check, surface::Surface};
#[path = "continuous/cell.rs"]
mod cell;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ContinuousOffsetLimits {
    pub max_cells: usize,
    pub max_depth: usize,
}
impl Default for ContinuousOffsetLimits {
    fn default() -> Self {
        Self {
            max_cells: 10000,
            max_depth: 24,
        }
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContinuousOffsetDecision {
    WithinTolerance,
    ExceedsTolerance,
    Unresolved,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ContinuousOffsetStop {
    Tolerance,
    Witness,
    WorkLimit,
    DepthLimit,
    PrecisionLimit,
}
#[derive(Clone, Debug, PartialEq)]
pub struct ContinuousOffsetReport {
    pub decision: ContinuousOffsetDecision,
    /// Only a complete cover supplies a whole-domain upper bound.
    pub error_upper: Option<f64>,
    pub error_lower: f64,
    pub witness_uv: Option<[f64; 2]>,
    /// Includes replaced parents, unsuccessful normal trials and all knot cells.
    pub cells: usize,
    pub accepted_cells: usize,
    pub stop: ContinuousOffsetStop,
    /// Regular one-sided normals on every covered original knot chart.
    pub source_normals_proven: bool,
    /// I - t*d*shape is positive definite for 0<=t<=1 on every source chart.
    pub ideal_local_fold_free: bool,
    /// Saved candidate has nonzero Jacobian with the original orientation.
    pub candidate_locally_regular: bool,
}
#[derive(Clone, Debug)]
pub struct QualifiedOffsetDeviation {
    source: Surface,
    candidate: Surface,
    distance: f64,
    tolerance: f64,
    limits: ContinuousOffsetLimits,
    report: ContinuousOffsetReport,
}
impl QualifiedOffsetDeviation {
    pub fn source(&self) -> &Surface {
        &self.source
    }
    pub fn candidate(&self) -> &Surface {
        &self.candidate
    }
    pub fn report(&self) -> &ContinuousOffsetReport {
        &self.report
    }
    pub fn reverify(&self) -> Result<()> {
        let fresh = inspect_report(
            &self.source,
            &self.candidate,
            self.distance,
            self.tolerance,
            self.limits,
        )?;
        check(
            fresh == self.report && fresh.decision == ContinuousOffsetDecision::WithinTolerance,
            "Original normal-offset deviation ownership changed",
        )
    }
}
pub struct ContinuousOffsetInspection {
    pub report: ContinuousOffsetReport,
    qualified: Option<QualifiedOffsetDeviation>,
}
impl ContinuousOffsetInspection {
    pub fn into_qualified(self) -> Option<QualifiedOffsetDeviation> {
        self.qualified
    }
}
/// Verify actual coefficients, never OffsetReport.exact, sampled max_deviation,
/// a primitive recognizer or an earlier certificate. Both UV domains must agree.
/// At interior knots the cover includes all incident original one-sided charts.
pub fn inspect_continuous_offset(
    source: &Surface,
    candidate: &Surface,
    distance: f64,
    tolerance: f64,
    limits: ContinuousOffsetLimits,
) -> Result<ContinuousOffsetInspection> {
    let report = inspect_report(source, candidate, distance, tolerance, limits)?;
    let qualified = (report.decision == ContinuousOffsetDecision::WithinTolerance).then(|| {
        QualifiedOffsetDeviation {
            source: source.clone(),
            candidate: candidate.clone(),
            distance,
            tolerance,
            limits,
            report: report.clone(),
        }
    });
    Ok(ContinuousOffsetInspection { report, qualified })
}
fn domain(s: &Surface) -> [[f64; 2]; 2] {
    [
        [s.knots_u[s.degree_u], s.knots_u[s.control_points.len()]],
        [s.knots_v[s.degree_v], s.knots_v[s.control_points[0].len()]],
    ]
}
fn breaks(a: &Surface, b: &Surface, axis: usize, d: [f64; 2]) -> Vec<f64> {
    let mut out = [a, b]
        .into_iter()
        .flat_map(|s| if axis == 0 { &s.knots_u } else { &s.knots_v })
        .copied()
        .filter(|v| d[0] <= *v && *v <= d[1])
        .collect::<Vec<_>>();
    out.sort_by(f64::total_cmp);
    out.dedup();
    out
}
fn span(s: &Surface, axis: usize, range: [f64; 2]) -> usize {
    let (p, n, k) = if axis == 0 {
        (s.degree_u, s.control_points.len(), &s.knots_u)
    } else {
        (s.degree_v, s.control_points[0].len(), &s.knots_v)
    };
    (p..n)
        .find(|&i| k[i] < k[i + 1] && k[i] <= range[0] && range[1] <= k[i + 1])
        .unwrap()
}
fn inspect_report(
    source: &Surface,
    candidate: &Surface,
    distance: f64,
    tolerance: f64,
    limits: ContinuousOffsetLimits,
) -> Result<ContinuousOffsetReport> {
    source.validate()?;
    candidate.validate()?;
    check(
        source.control_points[0][0].len() == 3 && candidate.control_points[0][0].len() == 3,
        "Continuous surface offset requires 3D sources",
    )?;
    check(
        distance.is_finite() && tolerance.is_finite() && tolerance > 0.,
        "Invalid continuous offset distance/tolerance",
    )?;
    check(
        limits.max_cells <= 100000 && limits.max_depth <= 40,
        "Continuous offset work limits exceeded",
    )?;
    let d = domain(source);
    check(
        d == domain(candidate),
        "Continuous offset needs the same original UV domain",
    )?;
    let mut out = ContinuousOffsetReport {
        decision: ContinuousOffsetDecision::Unresolved,
        error_upper: None,
        error_lower: 0.,
        witness_uv: None,
        cells: 0,
        accepted_cells: 0,
        stop: ContinuousOffsetStop::WorkLimit,
        source_normals_proven: false,
        ideal_local_fold_free: false,
        candidate_locally_regular: false,
    };
    if limits.max_cells == 0 {
        return Ok(out);
    }
    // Exact identity says nothing about source normals or embedding.
    if distance == 0. && source == candidate {
        out.decision = ContinuousOffsetDecision::WithinTolerance;
        out.error_upper = Some(0.);
        out.stop = ContinuousOffsetStop::Tolerance;
        return Ok(out);
    }
    let u = breaks(source, candidate, 0, d[0]);
    let v = breaks(source, candidate, 1, d[1]);
    let initial = (u.len() - 1).checked_mul(v.len() - 1).unwrap_or(usize::MAX);
    if initial > limits.max_cells {
        return Ok(out);
    }
    let mut pending = Vec::with_capacity(initial);
    for x in u.windows(2) {
        for y in v.windows(2) {
            let rect = [[x[0], x[1]], [y[0], y[1]]];
            pending.push((
                rect,
                [span(source, 0, rect[0]), span(source, 1, rect[1])],
                [span(candidate, 0, rect[0]), span(candidate, 1, rect[1])],
                0,
            ));
        }
    }
    let mut upper = 0_f64;
    let mut folded = false;
    let mut irregular = false;
    while let Some((rect, si, ci, depth)) = pending.pop() {
        if out.cells == limits.max_cells {
            return Ok(out);
        }
        out.cells += 1;
        let proof = cell::inspect(source, candidate, si, ci, rect, distance)?;
        if let Some(p) = &proof {
            out.error_lower = out.error_lower.max(p.lower);
            if p.lower > tolerance {
                out.decision = ContinuousOffsetDecision::ExceedsTolerance;
                out.witness_uv = Some(p.center);
                out.stop = ContinuousOffsetStop::Witness;
                return Ok(out);
            }
            if p.upper <= tolerance {
                upper = upper.max(p.upper);
                out.accepted_cells += 1;
                folded |= !p.fold_free;
                irregular |= !p.candidate_regular;
                continue;
            }
        }
        if depth == limits.max_depth {
            out.stop = ContinuousOffsetStop::DepthLimit;
            return Ok(out);
        }
        let axis = usize::from(
            (rect[1][1] - rect[1][0]) / (d[1][1] - d[1][0])
                > (rect[0][1] - rect[0][0]) / (d[0][1] - d[0][0]),
        );
        let mid = rect[axis][0] * 0.5 + rect[axis][1] * 0.5;
        if !(rect[axis][0] < mid && mid < rect[axis][1]) {
            out.stop = ContinuousOffsetStop::PrecisionLimit;
            return Ok(out);
        }
        let mut left = rect;
        left[axis][1] = mid;
        let mut right = rect;
        right[axis][0] = mid;
        pending.push((right, si, ci, depth + 1));
        pending.push((left, si, ci, depth + 1));
    }
    out.decision = ContinuousOffsetDecision::WithinTolerance;
    out.error_upper = Some(upper);
    out.stop = ContinuousOffsetStop::Tolerance;
    out.source_normals_proven = true;
    out.ideal_local_fold_free = !folded;
    out.candidate_locally_regular = !irregular;
    Ok(out)
}
impl super::OffsetReport {
    pub fn inspect_continuous(
        &self,
        source: &Surface,
        distance: f64,
        tolerance: f64,
        limits: ContinuousOffsetLimits,
    ) -> Result<ContinuousOffsetInspection> {
        inspect_continuous_offset(source, &self.surface, distance, tolerance, limits)
    }
}
