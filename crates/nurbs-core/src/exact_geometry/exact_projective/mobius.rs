//! Exact endpoint-preserving Möbius clocks for clamped Bézier charts.
use super::*;
use crate::exact_geometry::{rational_edit, rational_factor};
use num_traits::One;

/// Replace the unit curve parameter t by λt/(1+(λ-1)t). The chart retains its
/// absolute endpoint coordinates and the same image and orientation. The
/// Bernstein denominator acquires one common factor; stored weights are
/// scaled by λ^i up to one exact positive gauge. Supports 2D and 3D curves.
pub fn mobius_bezier_curve(source: &Curve, lambda: f64, max_work: u64) -> Result<Report<Curve>> {
    source.validate()?;
    validate_clock(&[lambda], max_work)?;
    validate_bezier(
        source.degree,
        &source.knots,
        source.control_points.len(),
        source.periodic,
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| {
        let powers = powers(lambda, source.degree, &mut work)?;
        let weights = source
            .weights
            .iter()
            .zip(powers)
            .map(|(&w, scale)| weighted(w, &scale, &mut work))
            .collect::<PResult<Vec<_>>>()?;
        let mut out = source.clone();
        out.weights = exact_weights(&weights, &mut work)?;
        out.validate().map_err(|_| Refusal::InvalidOutput)?;
        Ok(out)
    })();
    Ok(finish(result, work))
}

/// Apply independent endpoint-preserving Möbius clocks in U and V to a
/// single Bézier patch. One exact gauge covers the whole tensor net, with
/// positive weight factors λ_u^i λ_v^j. Cartesian poles and domains stay
/// literal; the interior UV correspondence follows the requested clocks.
pub fn mobius_bezier_surface(
    source: &Surface,
    lambda: [f64; 2],
    max_work: u64,
) -> Result<Report<Surface>> {
    source.validate()?;
    validate_clock(&lambda, max_work)?;
    validate_bezier(
        source.degree_u,
        &source.knots_u,
        source.control_points.len(),
        source.periodic_u,
    )?;
    validate_bezier(
        source.degree_v,
        &source.knots_v,
        source.control_points[0].len(),
        source.periodic_v,
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| {
        let u = powers(lambda[0], source.degree_u, &mut work)?;
        let v = powers(lambda[1], source.degree_v, &mut work)?;
        let mut weights = Vec::new();
        for (row, u) in source.weights.iter().zip(u) {
            for (&value, v) in row.iter().zip(&v) {
                let scale = work.op(&u, v, |a, b| a * b)?;
                weights.push(weighted(value, &scale, &mut work)?);
            }
        }
        let mut out = source.clone();
        out.weights = exact_weights(&weights, &mut work)?
            .chunks(source.degree_v + 1)
            .map(<[f64]>::to_vec)
            .collect();
        out.validate().map_err(|_| Refusal::InvalidOutput)?;
        Ok(out)
    })();
    Ok(finish(result, work))
}

pub(super) fn validate_clock(lambda: &[f64], max_work: u64) -> Result<()> {
    check(
        lambda.iter().all(|&t| t.is_finite() && t > 0.),
        "Möbius clock factors must be finite and positive",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Möbius clock work must be in [1,1000000]",
    )
}

fn validate_bezier(degree: usize, knots: &[f64], count: usize, periodic: bool) -> Result<()> {
    check(
        !periodic
            && count == degree + 1
            && knots[..=degree].iter().all(|&k| k == knots[0])
            && knots[count..].iter().all(|&k| k == knots[count]),
        "Möbius clocks require a single clamped nonperiodic Bézier chart",
    )
}

fn powers(lambda: f64, degree: usize, work: &mut Work) -> PResult<Vec<Q>> {
    let lambda = work.lift(lambda)?;
    let mut values = vec![Q::one()];
    for i in 0..degree {
        values.push(work.op(&values[i], &lambda, |a, b| a * b)?);
    }
    Ok(values)
}

fn weighted(weight: f64, scale: &Q, work: &mut Work) -> PResult<Q> {
    let weight = work.lift(weight)?;
    Ok(work.op(&weight, scale, |a, b| a * b)?)
}

pub(super) fn exact_weights(weights: &[Q], work: &mut Work) -> PResult<Vec<f64>> {
    let (_, stored) =
        rational_edit::store(&[weights.to_vec()], work).map_err(|reason| match reason {
            rational_factor::Refusal::ResourceLimit => Refusal::ResourceLimit,
            rational_factor::Refusal::NotRepresentable => Refusal::NotRepresentable,
            _ => Refusal::InvalidOutput,
        })?;
    let first = work.lift(stored[0])?;
    for (&stored, proposed) in stored.iter().zip(weights) {
        let stored = work.lift(stored)?;
        let a = work.op(&stored, &weights[0], |a, b| a * b)?;
        let b = work.op(&first, proposed, |a, b| a * b)?;
        if a != b {
            return Err(Refusal::UnprovedIdentity);
        }
    }
    Ok(stored)
}
