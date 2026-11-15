//! Exact fractional-linear clocks across complete nonperiodic knot vectors.
use super::{
    mobius::{exact_weights, validate_clock},
    *,
};
use num_traits::One;

struct ClockBasis {
    knots: Vec<f64>,
    factors: Vec<Q>,
}

/// Apply t ↦ λt/(1+(λ-1)t) in the unit domain of a clamped NURBS curve.
/// Each interior knot is pulled back exactly into the same absolute
/// interval. If a knot or the common-gauge weights cannot be stored in
/// binary64, return no curve.
pub fn mobius_spline_curve(source: &Curve, lambda: f64, max_work: u64) -> Result<Report<Curve>> {
    source.validate()?;
    validate_clock(&[lambda], max_work)?;
    clamped(
        source.degree,
        &source.knots,
        source.control_points.len(),
        source.periodic,
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let out = (|| {
        let basis = basis(
            &source.knots,
            source.degree,
            source.control_points.len(),
            lambda,
            &mut work,
        )?;
        let mut weights = Vec::with_capacity(source.weights.len());
        for (&weight, factor) in source.weights.iter().zip(basis.factors) {
            let weight = work.lift(weight)?;
            weights.push(work.op(&weight, &factor, |a, b| a * b)?);
        }
        let mut out = source.clone();
        out.knots = basis.knots;
        out.weights = exact_weights(&weights, &mut work)?;
        out.validate().map_err(|_| Refusal::InvalidOutput)?;
        Ok(out)
    })();
    Ok(finish(out, work))
}

/// Apply independent fractional-linear clocks to both absolute knot axes
/// of a clamped patch. All knot multiplicities and Cartesian poles remain,
/// and a single exact gauge covers the transformed tensor weights.
pub fn mobius_spline_surface(
    source: &Surface,
    lambda: [f64; 2],
    max_work: u64,
) -> Result<Report<Surface>> {
    source.validate()?;
    validate_clock(&lambda, max_work)?;
    let nu = source.control_points.len();
    let nv = source.control_points[0].len();
    clamped(source.degree_u, &source.knots_u, nu, source.periodic_u)?;
    clamped(source.degree_v, &source.knots_v, nv, source.periodic_v)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let out = (|| {
        let u = basis(&source.knots_u, source.degree_u, nu, lambda[0], &mut work)?;
        let v = basis(&source.knots_v, source.degree_v, nv, lambda[1], &mut work)?;
        let mut weights = Vec::with_capacity(nu * nv);
        for (row, u_factor) in source.weights.iter().zip(u.factors) {
            for (&weight, v_factor) in row.iter().zip(&v.factors) {
                let factor = work.op(&u_factor, v_factor, |a, b| a * b)?;
                let weight = work.lift(weight)?;
                weights.push(work.op(&weight, &factor, |a, b| a * b)?);
            }
        }
        let mut out = source.clone();
        out.knots_u = u.knots;
        out.knots_v = v.knots;
        out.weights = exact_weights(&weights, &mut work)?
            .chunks(nv)
            .map(<[f64]>::to_vec)
            .collect();
        out.validate().map_err(|_| Refusal::InvalidOutput)?;
        Ok(out)
    })();
    Ok(finish(out, work))
}

fn clamped(p: usize, knots: &[f64], count: usize, periodic: bool) -> Result<()> {
    check(
        !periodic
            && knots[..=p].iter().all(|&k| k == knots[p])
            && knots[count..].iter().all(|&k| k == knots[count]),
        "Möbius spline clocks require a clamped nonperiodic axis",
    )
}

fn basis(
    knots: &[f64],
    degree: usize,
    count: usize,
    lambda: f64,
    w: &mut Work,
) -> PResult<ClockBasis> {
    let a = w.lift(knots[degree])?;
    let b = w.lift(knots[count])?;
    let width = w.op(&b, &a, |a, b| a - b)?;
    let lambda = w.lift(lambda)?;
    let one = Q::one();
    let c = w.op(&lambda, &one, |a, b| a - b)?;
    let mut transformed = Vec::with_capacity(knots.len());
    let mut denominators = Vec::with_capacity(knots.len());
    for &k in knots {
        if k == knots[degree] {
            transformed.push(k);
            denominators.push(one.clone());
        } else if k == knots[count] {
            transformed.push(k);
            denominators.push(lambda.clone());
        } else {
            let old = w.lift(k)?;
            let old = w.op(&old, &a, |a, b| a - b)?;
            let t = w.op(&old, &width, |a, b| a / b)?;
            let ct = w.op(&c, &t, |a, b| a * b)?;
            let inverse_weight = w.op(&lambda, &ct, |a, b| a - b)?;
            let new_t = w.op(&t, &inverse_weight, |a, b| a / b)?;
            let absolute = w.op(&new_t, &width, |a, b| a * b)?;
            let absolute = w.op(&absolute, &a, |a, b| a + b)?;
            transformed.push(w.float(&absolute)?);
            let ct = w.op(&c, &new_t, |a, b| a * b)?;
            denominators.push(w.op(&one, &ct, |a, b| a + b)?);
        }
    }
    // Cox–de Boor recurrence: each level adds the denominator at the
    // missing knot. B_i^p(f(t)) = B'_i^p(t) Π D(k'_{i+j}) / D(t)^p.
    let mut factors = Vec::with_capacity(count);
    for i in 0..count {
        let mut product = one.clone();
        for d in &denominators[i + 1..=i + degree] {
            product = w.op(&product, d, |a, b| a * b)?;
        }
        factors.push(product);
    }
    Ok(ClockBasis {
        knots: transformed,
        factors,
    })
}
