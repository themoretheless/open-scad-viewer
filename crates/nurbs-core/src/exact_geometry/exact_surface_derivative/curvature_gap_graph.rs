//! Certify a whole-cell curvature threshold contour as one coordinate graph.
use super::{curvature_gap_threshold::threshold_residual, *};
use num_traits::Signed;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CurvatureGapGraph {
    /// Exactly one simple crossing for each coordinate on the other axis.
    /// The flag gives the sign of the residual slope in the chosen axis.
    Unique {
        increasing: bool,
    },
    EverywhereAtLevel,
    NoContour,
    Unresolved,
}

#[derive(Clone, Debug)]
pub struct CurvatureGapGraphAudit {
    pub domain: [[f64; 2]; 2],
    pub graph: Option<CurvatureGapGraph>,
    pub refusal: Option<Refusal>,
    pub work_used: u64,
}

/// Certify the squared-gap contour on an original UV cell as a graph of
/// the chosen axis over the other axis. Requires uniformly opposite signs
/// on the two fixed-coordinate faces and a strictly monotone residual in
/// the chosen axis. The certificate is local to this cell and does not
/// join traces across source knots.
pub fn certify_curvature_gap_graph(
    source: &Surface,
    span: [usize; 2],
    axis: Axis,
    level: f64,
    max_work: u64,
) -> Result<CurvatureGapGraphAudit> {
    let domain = validate_request(source, span, level, max_work)?;
    let [ku, kv] = span;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| {
        let level = work.lift(level)?;
        let residual = threshold_residual(source, ku, kv, &level, &mut work)?;
        classify(&residual, axis, &mut work)
    })();
    let (graph, refusal) = match result {
        Ok(graph) => (Some(graph), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(CurvatureGapGraphAudit {
        domain,
        graph,
        refusal,
        work_used: work.used,
    })
}

pub(super) fn validate_request(
    source: &Surface,
    span: [usize; 2],
    level: f64,
    max_work: u64,
) -> Result<[[f64; 2]; 2]> {
    source.validate()?;
    check(
        level.is_finite() && level > 0.,
        "Curvature gap level must be finite and positive",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Curvature graph work must be in [1,1000000]",
    )?;
    let [ku, kv] = span;
    check(
        (source.degree_u..source.control_points.len()).contains(&ku)
            && (source.degree_v..source.control_points[0].len()).contains(&kv),
        "Curvature graph requires active knot indices",
    )?;
    let domain = [
        [source.knots_u[ku], source.knots_u[ku + 1]],
        [source.knots_v[kv], source.knots_v[kv + 1]],
    ];
    check(
        domain.iter().all(|[a, b]| a < b),
        "Curvature graph requires a nonempty original cell",
    )?;
    Ok(domain)
}

pub(super) fn classify(residual: &Poly, axis: Axis, work: &mut Work) -> PResult<CurvatureGapGraph> {
    use CurvatureGapGraph::*;
    if residual.is_empty() {
        return Ok(EverywhereAtLevel);
    }
    let (pu, pv) = storage::degrees(std::slice::from_ref(residual));
    if pu > 25 || pv > 25 {
        return Ok(Unresolved);
    }
    let net = storage::bernstein(residual, pu, pv, work)?;
    if net.iter().all(Signed::is_positive) || net.iter().all(Signed::is_negative) {
        return Ok(NoContour);
    }
    let (line_len, lines) = match axis {
        Axis::U => (pu + 1, pv + 1),
        Axis::V => (pv + 1, pu + 1),
    };
    let coefficient = |line, i| match axis {
        Axis::U => &net[i * (pv + 1) + line],
        Axis::V => &net[line * (pv + 1) + i],
    };
    let increasing = coefficient(0, 0).is_negative();
    for line in 0..lines {
        let first = coefficient(line, 0);
        let last = coefficient(line, line_len - 1);
        let opposite = if increasing {
            first.is_negative() && last.is_positive()
        } else {
            first.is_positive() && last.is_negative()
        };
        if !opposite {
            return Ok(Unresolved);
        }
        // Differentiating a Bernstein polynomial multiplies successive
        // coefficient differences by the positive degree.
        work.charge((line_len - 1) as u64)?;
        for i in 0..line_len - 1 {
            let a = coefficient(line, i);
            let b = coefficient(line, i + 1);
            if (increasing && b <= a) || (!increasing && b >= a) {
                return Ok(Unresolved);
            }
        }
    }
    Ok(Unique { increasing })
}

#[derive(Clone, Debug, PartialEq)]
pub struct CurvatureGapSample {
    /// The caller's absolute parameter on the transverse axis.
    pub transverse: f64,
    /// Certified closed interval in the root axis's unit coordinate.
    /// Equal endpoints identify an exact dyadic root.
    pub root_interval: [f64; 2],
}

#[derive(Clone, Debug)]
pub struct CurvatureGapSamples {
    pub domain: [[f64; 2]; 2],
    pub graph: Option<CurvatureGapGraph>,
    pub samples: Option<Vec<CurvatureGapSample>>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Sample a certified unique curvature threshold contour in request order.
/// The input transverse parameters are lifted exactly from their binary64
/// values. Each root bracket is at most 2^-max_depth of the original knot
/// span. A nonunique or unresolved graph yields no samples. Work refusal
/// discards all samples and the graph status.
pub fn sample_curvature_gap_graph(
    source: &Surface,
    span: [usize; 2],
    axis: Axis,
    level: f64,
    transverse: &[f64],
    max_depth: u8,
    max_work: u64,
) -> Result<CurvatureGapSamples> {
    let domain = validate_request(source, span, level, max_work)?;
    check(
        max_depth <= 16,
        "Curvature graph sample depth must be in [0,16]",
    )?;
    check(
        (1..=256).contains(&transverse.len()),
        "Curvature graph sample count must be in [1,256]",
    )?;
    let root_axis = match axis {
        Axis::U => 0,
        Axis::V => 1,
    };
    let [start, end] = domain[1 - root_axis];
    check(
        transverse.iter().all(|&p| p >= start && p <= end),
        "Curvature graph transverse parameters must lie inside the original cell",
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
            return Ok((graph, None));
        };
        let samples = sample_certified(
            &residual, domain, root_axis, increasing, transverse, max_depth, &mut work,
        )?;
        Ok((graph, Some(samples)))
    })();
    let (graph, samples, refusal) = match result {
        Ok((graph, samples)) => (Some(graph), samples, None),
        Err(reason) => (None, None, Some(reason)),
    };
    Ok(CurvatureGapSamples {
        domain,
        graph,
        samples,
        work_used: work.used,
        refusal,
    })
}

pub(super) fn sample_certified(
    residual: &Poly,
    domain: [[f64; 2]; 2],
    root_axis: usize,
    increasing: bool,
    transverse: &[f64],
    max_depth: u8,
    work: &mut Work,
) -> PResult<Vec<CurvatureGapSample>> {
    let [start, end] = domain[1 - root_axis];
    let start = work.lift(start)?;
    let end = work.lift(end)?;
    let width = work.op(&end, &start, |a, b| a - b)?;
    let mut samples = Vec::with_capacity(transverse.len());
    for &transverse in transverse {
        let value = work.lift(transverse)?;
        let offset = work.op(&value, &start, |a, b| a - b)?;
        let unit = work.op(&offset, &width, |a, b| a / b)?;
        let coefficients = restrict(residual, root_axis, &unit, work)?;
        let root_interval = bisect(&coefficients, increasing, max_depth, work)?;
        samples.push(CurvatureGapSample {
            transverse,
            root_interval,
        });
    }
    Ok(samples)
}

fn restrict(residual: &Poly, axis: usize, unit: &Q, work: &mut Work) -> PResult<Vec<Q>> {
    let (pu, pv) = storage::degrees(std::slice::from_ref(residual));
    let degrees = [pu, pv];
    let mut powers = vec![Q::one()];
    for i in 0..degrees[1 - axis] {
        powers.push(work.op(&powers[i], unit, |a, b| a * b)?);
    }
    let mut out = vec![Q::zero(); degrees[axis] + 1];
    for (&(u, v), coefficient) in residual {
        let exponents = [u, v];
        let term = work.op(coefficient, &powers[exponents[1 - axis]], |a, b| a * b)?;
        let index = exponents[axis];
        out[index] = work.op(&out[index], &term, |a, b| a + b)?;
    }
    Ok(out)
}

fn bisect(coefficients: &[Q], increasing: bool, depth: u8, work: &mut Work) -> PResult<[f64; 2]> {
    let mut interval = [0., 1.];
    for _ in 0..depth {
        let midpoint = (interval[0] + interval[1]) * 0.5;
        let x = work.lift(midpoint)?;
        let mut value = Q::zero();
        for coefficient in coefficients.iter().rev() {
            let term = work.op(&value, &x, |a, b| a * b)?;
            value = work.op(&term, coefficient, |a, b| a + b)?;
        }
        if value.is_zero() {
            return Ok([midpoint; 2]);
        }
        // The whole-cell graph proof provides strict opposite endpoint signs.
        // Keep the endpoint opposite to the sign at the new midpoint.
        interval[usize::from(value.is_positive() == increasing)] = midpoint;
    }
    Ok(interval)
}
