//! Signed second derivative of a unique threshold contour in its UV chart.
use super::{
    curvature_gap_graph::{CurvatureGapGraph, classify, validate_request},
    curvature_gap_slope::ratio_bounds,
    curvature_gap_threshold::threshold_residual,
    *,
};

#[derive(Clone, Debug)]
pub struct CurvatureGapSecondDerivativeAudit {
    pub domain: [[f64; 2]; 2],
    pub graph: Option<CurvatureGapGraph>,
    pub second_derivative: Option<[f64; 2]>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Bound the second derivative of the root parameter with respect to the
/// transverse parameter, in the original absolute knot clocks. Both bounds
/// hold on the entire certified branch. An exact constant ratio can
/// be retained even when its unreduced polynomials exceed the hull cap.
pub fn curvature_gap_graph_second_derivative(
    source: &Surface,
    span: [usize; 2],
    axis: Axis,
    level: f64,
    max_work: u64,
) -> Result<CurvatureGapSecondDerivativeAudit> {
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
        let bounds = second_bounds(&residual, domain, axis, increasing, &mut work)?;
        Ok((graph, bounds))
    })();
    let (graph, second_derivative, refusal) = match result {
        Ok((graph, bounds)) => (Some(graph), bounds, None),
        Err(reason) => (None, None, Some(reason)),
    };
    Ok(CurvatureGapSecondDerivativeAudit {
        domain,
        graph,
        second_derivative,
        refusal,
        work_used: work.used,
    })
}

pub(super) fn second_bounds(
    residual: &Poly,
    domain: [[f64; 2]; 2],
    axis: Axis,
    increasing: bool,
    work: &mut Work,
) -> PResult<Option<[f64; 2]>> {
    let root = match axis {
        Axis::U => 0,
        Axis::V => 1,
    };
    let other = 1 - root;
    let widths = |i: usize, work: &mut Work| -> PResult<Q> {
        let [a, b] = domain[i];
        let (a, b) = (work.lift(a)?, work.lift(b)?);
        Ok(work.op(&b, &a, |a, b| a - b)?)
    };
    let root_width = widths(root, work)?;
    let other_width = widths(other, work)?;
    let axes = [Axis::U, Axis::V];
    let d = polynomial::derivative(residual, axes[root], &root_width, work)?;
    let t = polynomial::derivative(residual, axes[other], &other_width, work)?;
    let rr = polynomial::derivative(&d, axes[root], &root_width, work)?;
    let rt = polynomial::derivative(&d, axes[other], &other_width, work)?;
    let tt = polynomial::derivative(&t, axes[other], &other_width, work)?;

    // u'' = (2 F_uv F_u F_v - F_uu F_v² - F_vv F_u²) / F_u³.
    let d2 = polynomial::multiply(&d, &d, work)?;
    let t2 = polynomial::multiply(&t, &t, work)?;
    let dt = polynomial::multiply(&d, &t, work)?;
    let mut cross = polynomial::multiply(&rt, &dt, work)?;
    let two = Q::from_integer(2.into());
    for coefficient in cross.values_mut() {
        *coefficient = work.op(coefficient, &two, |a, b| a * b)?;
    }
    let rr_t2 = polynomial::multiply(&rr, &t2, work)?;
    let tt_d2 = polynomial::multiply(&tt, &d2, work)?;
    let numerator = polynomial::subtract(polynomial::subtract(cross, &rr_t2, work)?, &tt_d2, work)?;
    let denominator = polynomial::multiply(&d2, &d, work)?;
    let target = [numerator, Poly::new(), Poly::new(), denominator];
    if let Some(constant) = polynomial::constant_field(&target, work)? {
        return ratio_bounds(&constant[0], &constant[3], true, work);
    }
    ratio_bounds(&target[0], &target[3], increasing, work)
}
