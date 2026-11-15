//! Independently reparameterize each Bézier span without a weight seam.
use super::{
    mobius::{exact_weights, validate_clock},
    *,
};
use num_traits::One;

#[derive(Clone, Copy, Debug)]
pub struct BezierSpanLayout<'a> {
    /// Ordered absolute endpoints, including both ends of the chart.
    pub breaks: &'a [f64],
    /// One positive Möbius factor per span.
    pub factors: &'a [f64],
}

/// Reclock each span of a clamped piecewise Bézier curve independently.
/// Interior knots must have degree multiplicity and stay at their original
/// coordinates. The seam weights share one exact global gauge. Geometric
/// continuity is preserved; derivative magnitudes may change across seams.
pub fn mobius_piecewise_bezier_curve(
    source: &Curve,
    lambda: &[f64],
    max_work: u64,
) -> Result<Report<Curve>> {
    apply_curve(source, lambda, None, max_work)
}

/// Assign new absolute span boundaries and independent local clocks while
/// retaining the original geometric image and one common seam gauge.
pub fn relayout_bezier_curve(
    source: &Curve,
    layout: BezierSpanLayout<'_>,
    max_work: u64,
) -> Result<Report<Curve>> {
    validate_layout(layout)?;
    apply_curve(source, layout.factors, Some(layout.breaks), max_work)
}

fn apply_curve(
    source: &Curve,
    lambda: &[f64],
    breaks: Option<&[f64]>,
    max_work: u64,
) -> Result<Report<Curve>> {
    source.validate()?;
    validate_clock(lambda, max_work)?;
    validate_spans(
        source.degree,
        &source.knots,
        source.control_points.len(),
        source.periodic,
        lambda,
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| {
        let factors = factors(source.degree, lambda, &mut work)?;
        let mut weights = Vec::with_capacity(factors.len());
        for (&weight, factor) in source.weights.iter().zip(factors) {
            let weight = work.lift(weight)?;
            weights.push(work.op(&weight, &factor, |a, b| a * b)?);
        }
        let mut out = source.clone();
        out.weights = exact_weights(&weights, &mut work)?;
        if let Some(breaks) = breaks {
            out.knots = expand_breaks(breaks, source.degree);
        }
        out.validate().map_err(|_| Refusal::InvalidOutput)?;
        Ok(out)
    })();
    Ok(finish(result, work))
}

/// Tensor counterpart with one positive factor per U or V Bézier span.
/// The assembled result retains all original knots and poles, with one
/// stored weight shared across each internal seam.
pub fn mobius_piecewise_bezier_surface(
    source: &Surface,
    lambda_u: &[f64],
    lambda_v: &[f64],
    max_work: u64,
) -> Result<Report<Surface>> {
    apply_surface(source, lambda_u, lambda_v, [None; 2], max_work)
}

/// Assign independent absolute U/V span layouts, preserving the original
/// tensor image and shared seam controls. Trims in the old UV coordinates
/// need the matching parameter map separately.
pub fn relayout_bezier_surface(
    source: &Surface,
    layouts: [BezierSpanLayout<'_>; 2],
    max_work: u64,
) -> Result<Report<Surface>> {
    let [u, v] = layouts;
    validate_layout(u)?;
    validate_layout(v)?;
    apply_surface(
        source,
        u.factors,
        v.factors,
        [Some(u.breaks), Some(v.breaks)],
        max_work,
    )
}

fn apply_surface(
    source: &Surface,
    lambda_u: &[f64],
    lambda_v: &[f64],
    breaks: [Option<&[f64]>; 2],
    max_work: u64,
) -> Result<Report<Surface>> {
    source.validate()?;
    validate_clock(lambda_u, max_work)?;
    validate_clock(lambda_v, max_work)?;
    let nu = source.control_points.len();
    let nv = source.control_points[0].len();
    validate_spans(
        source.degree_u,
        &source.knots_u,
        nu,
        source.periodic_u,
        lambda_u,
    )?;
    validate_spans(
        source.degree_v,
        &source.knots_v,
        nv,
        source.periodic_v,
        lambda_v,
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| {
        let u = factors(source.degree_u, lambda_u, &mut work)?;
        let v = factors(source.degree_v, lambda_v, &mut work)?;
        let mut weights = Vec::with_capacity(nu * nv);
        for (row, u) in source.weights.iter().zip(u) {
            for (&weight, v) in row.iter().zip(&v) {
                let factor = work.op(&u, v, |a, b| a * b)?;
                let weight = work.lift(weight)?;
                weights.push(work.op(&weight, &factor, |a, b| a * b)?);
            }
        }
        let mut out = source.clone();
        out.weights = exact_weights(&weights, &mut work)?
            .chunks(nv)
            .map(<[f64]>::to_vec)
            .collect();
        if let Some(breaks) = breaks[0] {
            out.knots_u = expand_breaks(breaks, source.degree_u);
        }
        if let Some(breaks) = breaks[1] {
            out.knots_v = expand_breaks(breaks, source.degree_v);
        }
        out.validate().map_err(|_| Refusal::InvalidOutput)?;
        Ok(out)
    })();
    Ok(finish(result, work))
}

fn validate_spans(
    degree: usize,
    knots: &[f64],
    count: usize,
    periodic: bool,
    lambda: &[f64],
) -> Result<()> {
    check(
        !periodic
            && knots[..=degree].iter().all(|&k| k == knots[degree])
            && knots[count..].iter().all(|&k| k == knots[count])
            && knots[degree + 1..count]
                .chunk_by(|a, b| a == b)
                .all(|run| run.len() == degree)
            && lambda.len().checked_mul(degree) == Some(count - 1),
        "Piecewise Möbius clocks need one factor per clamped Bézier span",
    )
}

fn factors(degree: usize, lambda: &[f64], work: &mut Work) -> PResult<Vec<Q>> {
    let mut factors = vec![Q::one()];
    let mut prefix = Q::one();
    for &lambda in lambda {
        let lambda = work.lift(lambda)?;
        for _ in 0..degree {
            prefix = work.op(&prefix, &lambda, |a, b| a * b)?;
            factors.push(prefix.clone());
        }
    }
    Ok(factors)
}

pub(super) fn validate_layout(layout: BezierSpanLayout<'_>) -> Result<()> {
    check(
        layout.factors.len().checked_add(1) == Some(layout.breaks.len())
            && !layout.factors.is_empty()
            && layout.breaks.iter().all(|k| k.is_finite())
            && layout.breaks.windows(2).all(|pair| pair[0] < pair[1]),
        "Bézier span layout requires one increasing finite interval per factor",
    )
}

fn expand_breaks(breaks: &[f64], degree: usize) -> Vec<f64> {
    let mut knots = vec![breaks[0]];
    for &knot in breaks {
        knots.extend(std::iter::repeat_n(knot, degree));
    }
    knots.push(breaks[breaks.len() - 1]);
    knots
}
