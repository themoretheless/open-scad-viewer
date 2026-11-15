//! Continuous absolute chart error for a polyline of a certified contour.
use super::{
    curvature_gap_graph::{
        CurvatureGapGraph, CurvatureGapSample, classify, sample_certified, validate_request,
    },
    curvature_gap_range::outward,
    curvature_gap_slope::slope_bounds,
    curvature_gap_threshold::threshold_residual,
    *,
};

#[derive(Clone, Debug)]
pub struct CurvatureGapPolyline {
    pub domain: [[f64; 2]; 2],
    pub graph: Option<CurvatureGapGraph>,
    /// Vertices are the midpoints of these exact unit-root brackets, at
    /// their stored absolute transverse parameters.
    pub samples: Option<Vec<CurvatureGapSample>>,
    /// Continuous error per segment in the absolute root parameter, at
    /// equal transverse coordinates. This is a UV chart bound.
    pub segment_error_upper: Option<Vec<f64>>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Certify a polyline covering an entire curvature-gap graph in one cell.
/// The transverse parameters must increase strictly and include both ends
/// of their absolute knot span. At most 256 samples share a single work
/// ledger with the graph, slope and continuous segment proofs.
pub fn certify_curvature_gap_polyline(
    source: &Surface,
    span: [usize; 2],
    axis: Axis,
    level: f64,
    transverse: &[f64],
    max_depth: u8,
    max_work: u64,
) -> Result<CurvatureGapPolyline> {
    let domain = validate_request(source, span, level, max_work)?;
    check(
        max_depth <= 16,
        "Curvature graph sample depth must be in [0,16]",
    )?;
    let root_axis = match axis {
        Axis::U => 0,
        Axis::V => 1,
    };
    let [a, b] = domain[1 - root_axis];
    check(
        (2..=256).contains(&transverse.len())
            && transverse.first() == Some(&a)
            && transverse.last() == Some(&b)
            && transverse.windows(2).all(|p| p[0] < p[1]),
        "Curvature polyline needs an ordered complete transverse partition of 2..256 points",
    )?;

    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| {
        let level = work.lift(level)?;
        let residual = threshold_residual(source, span[0], span[1], &level, &mut work)?;
        let graph = classify(&residual, axis, &mut work)?;
        let CurvatureGapGraph::Unique { increasing } = graph else {
            return Ok((graph, None, None));
        };
        let Some(slope) = slope_bounds(&residual, domain, axis, increasing, &mut work)? else {
            return Ok((graph, None, None));
        };
        let samples = sample_certified(
            &residual, domain, root_axis, increasing, transverse, max_depth, &mut work,
        )?;
        let Some(errors) = segment_errors(&samples, domain[root_axis], slope, &mut work)? else {
            return Ok((graph, None, None));
        };
        Ok((graph, Some(samples), Some(errors)))
    })();
    let (graph, samples, segment_error_upper, refusal) = match result {
        Ok((graph, samples, errors)) => (Some(graph), samples, errors, None),
        Err(reason) => (None, None, None, Some(reason)),
    };
    Ok(CurvatureGapPolyline {
        domain,
        graph,
        samples,
        segment_error_upper,
        work_used: work.used,
        refusal,
    })
}

fn width([a, b]: [f64; 2], work: &mut Work) -> PResult<Q> {
    let a = work.lift(a)?;
    let b = work.lift(b)?;
    Ok(work.op(&b, &a, |a, b| a - b)?)
}

fn segment_errors(
    samples: &[CurvatureGapSample],
    root_domain: [f64; 2],
    slope: [f64; 2],
    work: &mut Work,
) -> PResult<Option<Vec<f64>>> {
    let root_width = width(root_domain, work)?;
    let slope_width = width(slope, work)?;
    let mut errors = Vec::with_capacity(samples.len() - 1);
    for pair in samples.windows(2) {
        let h = width([pair[0].transverse, pair[1].transverse], work)?;
        let a = width(pair[0].root_interval, work)?;
        let b = width(pair[1].root_interval, work)?;
        // The exact chord's interpolation remainder is bounded by
        // h (M-m)/4: the slopes left and right of each query differ
        // by at most M-m, multiplied by lambda(1-lambda) h.
        let chord = work.op(&h, &slope_width, |a, b| a * b / Q::from_integer(4.into()))?;
        let endpoints = work.op(&root_width, &a.max(b), |a, b| {
            a * b / Q::from_integer(2.into())
        })?;
        let upper = work.op(&chord, &endpoints, |a, b| a + b)?;
        let Some(upper) = outward(&upper, true, work)? else {
            return Ok(None);
        };
        errors.push(upper);
    }
    Ok(Some(errors))
}
