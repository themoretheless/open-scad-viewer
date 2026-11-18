//! Continuous absolute chart error for a polyline of a certified contour.
use super::{
    curvature_gap_graph::{
        CurvatureGapGraph, CurvatureGapSample, classify, sample_certified, validate_request,
    },
    curvature_gap_range::outward,
    curvature_gap_second::second_bounds,
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
        let Some(upper) = segment_error(&pair[0], &pair[1], &root_width, &slope_width, None, work)?
        else {
            return Ok(None);
        };
        errors.push(upper);
    }
    Ok(Some(errors))
}

fn segment_error(
    a: &CurvatureGapSample,
    b: &CurvatureGapSample,
    root_width: &Q,
    slope_width: &Q,
    second_abs: Option<&Q>,
    work: &mut Work,
) -> PResult<Option<f64>> {
    let h = width([a.transverse, b.transverse], work)?;
    let a = width(a.root_interval, work)?;
    let b = width(b.root_interval, work)?;
    // Left and right secant slopes differ by at most M-m, multiplied
    // by lambda(1-lambda) h. Add the maximum endpoint rounding error.
    let chord = work.op(&h, slope_width, |a, b| a * b / Q::from_integer(4.into()))?;
    let chord = if let Some(second_abs) = second_abs {
        // The linear interpolant remainder is bounded by max|u''| h²/8.
        let h_squared = work.op(&h, &h, |a, b| a * b)?;
        chord.min(work.op(&h_squared, second_abs, |a, b| {
            a * b / Q::from_integer(8.into())
        })?)
    } else {
        chord
    };
    let endpoints = work.op(root_width, &a.max(b), |a, b| {
        a * b / Q::from_integer(2.into())
    })?;
    let upper = work.op(&chord, &endpoints, |a, b| a + b)?;
    outward(&upper, true, work)
}

#[derive(Clone, Copy, Debug)]
pub struct AdaptiveCurvaturePolylineOptions {
    pub max_error: f64,
    pub max_vertices: usize,
    pub root_depth: u8,
    pub max_work: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurvaturePolylineStop {
    ToleranceMet,
    VertexLimit,
    ParameterPrecision,
}

#[derive(Clone, Debug)]
pub struct AdaptiveCurvaturePolyline {
    pub polyline: CurvatureGapPolyline,
    /// None when a complete finite polyline certificate could not be made.
    pub stop: Option<CurvaturePolylineStop>,
}

/// Refine a unique contour's UV polyline against an absolute root-axis
/// tolerance. A vertex or parameter-precision limit retains a complete
/// certified polyline and its actual segment bounds. Exact work refusal
/// discards the partial result.
pub fn adapt_curvature_gap_polyline(
    source: &Surface,
    span: [usize; 2],
    axis: Axis,
    level: f64,
    options: AdaptiveCurvaturePolylineOptions,
) -> Result<AdaptiveCurvaturePolyline> {
    adapt(source, span, axis, level, options, false)
}

/// Refine using the tighter of the slope and second derivative chord
/// bounds wherever the second derivative has a finite certificate.
/// The requested tolerance is in the absolute root parameter.
pub fn adapt_curvature_gap_polyline_second_order(
    source: &Surface,
    span: [usize; 2],
    axis: Axis,
    level: f64,
    options: AdaptiveCurvaturePolylineOptions,
) -> Result<AdaptiveCurvaturePolyline> {
    adapt(source, span, axis, level, options, true)
}

fn adapt(
    source: &Surface,
    span: [usize; 2],
    axis: Axis,
    level: f64,
    options: AdaptiveCurvaturePolylineOptions,
    use_second: bool,
) -> Result<AdaptiveCurvaturePolyline> {
    use CurvaturePolylineStop::*;
    let domain = validate_request(source, span, level, options.max_work)?;
    check(
        options.max_error.is_finite() && options.max_error > 0.,
        "Curvature polyline tolerance must be finite and positive",
    )?;
    check(
        (2..=256).contains(&options.max_vertices),
        "Curvature polyline vertex budget must be in [2,256]",
    )?;
    check(
        options.root_depth <= 16,
        "Curvature graph sample depth must be in [0,16]",
    )?;
    let root_axis = match axis {
        Axis::U => 0,
        Axis::V => 1,
    };
    let mut work = Work {
        used: 0,
        limit: options.max_work,
    };
    let result = (|| {
        let level = work.lift(level)?;
        let residual = threshold_residual(source, span[0], span[1], &level, &mut work)?;
        let graph = classify(&residual, axis, &mut work)?;
        let CurvatureGapGraph::Unique { increasing } = graph else {
            return Ok((graph, None));
        };
        let Some(slope) = slope_bounds(&residual, domain, axis, increasing, &mut work)? else {
            return Ok((graph, None));
        };
        let root_width = width(domain[root_axis], &mut work)?;
        let slope_width = width(slope, &mut work)?;
        let second_abs = if use_second {
            second_bounds(&residual, domain, axis, increasing, &mut work)?
                .map(|[lo, hi]| work.lift(lo.abs().max(hi.abs())))
                .transpose()?
        } else {
            None
        };
        let sample = |parameters: &[f64], work: &mut Work| {
            sample_certified(
                &residual,
                domain,
                root_axis,
                increasing,
                parameters,
                options.root_depth,
                work,
            )
        };
        let endpoints = sample(&domain[1 - root_axis], &mut work)?;
        let mut samples = vec![endpoints[0].clone()];
        let mut pending = vec![(endpoints[0].clone(), endpoints[1].clone())];
        let mut errors = Vec::new();
        let mut vertices = 2;
        let mut stop = ToleranceMet;
        while let Some((a, b)) = pending.pop() {
            let Some(error) = segment_error(
                &a,
                &b,
                &root_width,
                &slope_width,
                second_abs.as_ref(),
                &mut work,
            )?
            else {
                return Ok((graph, None));
            };
            if error > options.max_error {
                let midpoint = a.transverse.midpoint(b.transverse);
                if vertices == options.max_vertices {
                    stop = VertexLimit;
                } else if midpoint <= a.transverse || midpoint >= b.transverse {
                    if stop != VertexLimit {
                        stop = ParameterPrecision;
                    }
                } else {
                    let middle = sample(&[midpoint], &mut work)?.remove(0);
                    vertices += 1;
                    pending.push((middle.clone(), b));
                    pending.push((a, middle));
                    continue;
                }
            }
            errors.push(error);
            samples.push(b);
        }
        Ok((graph, Some((samples, errors, stop))))
    })();
    let (graph, samples, segment_error_upper, stop, refusal) = match result {
        Ok((graph, Some((samples, errors, stop)))) => {
            (Some(graph), Some(samples), Some(errors), Some(stop), None)
        }
        Ok((graph, None)) => (Some(graph), None, None, None, None),
        Err(reason) => (None, None, None, None, Some(reason)),
    };
    Ok(AdaptiveCurvaturePolyline {
        polyline: CurvatureGapPolyline {
            domain,
            graph,
            samples,
            segment_error_upper,
            work_used: work.used,
            refusal,
        },
        stop,
    })
}
