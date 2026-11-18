//! Exact coefficient-ratio bounds for a certified contour's chart slope.
use super::{
    curvature_gap_graph::{CurvatureGapGraph, classify, validate_request},
    curvature_gap_range::outward,
    curvature_gap_threshold::threshold_residual,
    *,
};
use num_traits::Signed;

#[derive(Clone, Debug)]
pub struct CurvatureGapSlopeAudit {
    pub domain: [[f64; 2]; 2],
    pub graph: Option<CurvatureGapGraph>,
    /// Outward bounds on d(root parameter)/d(transverse parameter) in the
    /// absolute knot clocks. Only a certified graph can produce a bound.
    pub slope: Option<[f64; 2]>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Bound the slope of the unique curvature threshold contour across the
/// whole original UV cell using implicit differentiation. The strictly
/// monotone residual supplies the nonzero denominator before any ratio.
/// A graph whose slope cannot be stored in finite bounds retains its status.
pub fn curvature_gap_graph_slope(
    source: &Surface,
    span: [usize; 2],
    axis: Axis,
    level: f64,
    max_work: u64,
) -> Result<CurvatureGapSlopeAudit> {
    let domain = validate_request(source, span, level, max_work)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| {
        let level = work.lift(level)?;
        let residual = threshold_residual(source, span[0], span[1], &level, &mut work)?;
        let graph = classify(&residual, axis, &mut work)?;
        let CurvatureGapGraph::Unique { increasing } = graph else {
            return Ok((graph, None));
        };
        Ok((
            graph,
            slope_bounds(&residual, domain, axis, increasing, &mut work)?,
        ))
    })();
    let (graph, slope, refusal) = match result {
        Ok((graph, slope)) => (Some(graph), slope, None),
        Err(reason) => (None, None, Some(reason)),
    };
    Ok(CurvatureGapSlopeAudit {
        domain,
        graph,
        slope,
        work_used: work.used,
        refusal,
    })
}

pub(super) fn slope_bounds(
    residual: &Poly,
    domain: [[f64; 2]; 2],
    axis: Axis,
    increasing: bool,
    work: &mut Work,
) -> PResult<Option<[f64; 2]>> {
    let (root, transverse) = match axis {
        Axis::U => (0, 1),
        Axis::V => (1, 0),
    };
    let width = |axis: usize, work: &mut Work| -> PResult<Q> {
        let [a, b] = domain[axis];
        let (a, b) = (work.lift(a)?, work.lift(b)?);
        Ok(work.op(&b, &a, |a, b| a - b)?)
    };
    let axes = [Axis::U, Axis::V];
    let d_root = polynomial::derivative(residual, axes[root], &width(root, work)?, work)?;
    let d_transverse =
        polynomial::derivative(residual, axes[transverse], &width(transverse, work)?, work)?;
    let zero = Poly::new();
    let numerator = polynomial::subtract(zero, &d_transverse, work)?;
    ratio_bounds(&numerator, &d_root, increasing, work)
}

pub(super) fn ratio_bounds(
    numerator: &Poly,
    denominator: &Poly,
    increasing: bool,
    work: &mut Work,
) -> PResult<Option<[f64; 2]>> {
    let (pu, pv) = (
        numerator
            .keys()
            .chain(denominator.keys())
            .map(|k| k.0)
            .max()
            .unwrap_or(0)
            .max(1),
        numerator
            .keys()
            .chain(denominator.keys())
            .map(|k| k.1)
            .max()
            .unwrap_or(0)
            .max(1),
    );
    if pu > 25 || pv > 25 {
        return Ok(None);
    }
    let denominator = storage::bernstein(denominator, pu, pv, work)?;
    let numerator = storage::bernstein(numerator, pu, pv, work)?;
    if !denominator.iter().all(|q| {
        if increasing {
            q.is_positive()
        } else {
            q.is_negative()
        }
    }) {
        return Ok(None);
    }
    // Equal-degree Bernstein nets give a convex combination of the
    // ratios even when all denominator coefficients are negative.
    let mut extrema: Option<[Q; 2]> = None;
    for (n, d) in numerator.iter().zip(&denominator) {
        let ratio = work.op(n, d, |n, d| n / d)?;
        extrema = Some(match extrema {
            Some([lo, hi]) => [lo.min(ratio.clone()), hi.max(ratio)],
            None => [ratio.clone(), ratio],
        });
    }
    let Some([lo, hi]) = extrema else {
        return Ok(None);
    };
    Ok(outward(&lo, false, work)?
        .zip(outward(&hi, true, work)?)
        .map(|(lo, hi)| [lo, hi]))
}
