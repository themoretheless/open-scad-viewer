//! Independently reparameterize each Bézier span without a weight seam.
use super::{
    mobius::{exact_weights, validate_clock},
    *,
};
use num_traits::One;

/// Reclock each span of a clamped piecewise Bézier curve independently.
/// Interior knots must have degree multiplicity and stay at their original
/// coordinates. The seam weights share one exact global gauge. Geometric
/// continuity is preserved; derivative magnitudes may change across seams.
pub fn mobius_piecewise_bezier_curve(
    source: &Curve,
    lambda: &[f64],
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
