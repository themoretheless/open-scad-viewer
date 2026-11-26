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

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MobiusLandmark {
    pub original: f64,
    pub new: f64,
}

#[derive(Clone, Copy)]
enum Clock {
    Factor(f64),
    Landmark(MobiusLandmark),
}

/// Apply t ↦ λt/(1+(λ-1)t) in the unit domain of a clamped NURBS curve.
/// Each interior knot is pulled back exactly into the same absolute
/// interval. If a knot or the common-gauge weights cannot be stored in
/// binary64, return no curve.
pub fn mobius_spline_curve(source: &Curve, lambda: f64, max_work: u64) -> Result<Report<Curve>> {
    source.validate()?;
    validate_clock(&[lambda], max_work)?;
    apply_curve(source, Clock::Factor(lambda), max_work)
}

/// Reclock a curve so the requested new interior parameter evaluates at
/// the original landmark. Both domain endpoints stay fixed. The fitted
/// fractional-linear factor is kept exact, even if it is not binary64.
pub fn fit_mobius_spline_curve(
    source: &Curve,
    landmark: MobiusLandmark,
    max_work: u64,
) -> Result<Report<Curve>> {
    source.validate()?;
    validate_clock(&[], max_work)?;
    validate_landmark(landmark, source.domain())?;
    apply_curve(source, Clock::Landmark(landmark), max_work)
}

fn apply_curve(source: &Curve, clock: Clock, max_work: u64) -> Result<Report<Curve>> {
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
            clock,
            &mut work,
        )?;
        reweight_curve(source, basis, &mut work)
    })();
    Ok(finish(out, work))
}

fn reweight_curve(source: &Curve, basis: ClockBasis, work: &mut Work) -> PResult<Curve> {
    let mut weights = Vec::with_capacity(source.weights.len());
    for (&weight, factor) in source.weights.iter().zip(basis.factors) {
        let weight = work.lift(weight)?;
        weights.push(work.op(&weight, &factor, |a, b| a * b)?);
    }
    let mut out = source.clone();
    out.knots = basis.knots;
    out.weights = exact_weights(&weights, work)?;
    out.validate().map_err(|_| Refusal::InvalidOutput)?;
    Ok(out)
}

/// Map an old offset x to speed*x/(1-c*x), measured from the new start.
/// For an increasing pole-free map the inverse has denominator 1/(1-c*x)
/// at each transformed knot, in the normalization with start weight one.
pub(crate) fn match_curve_seam_clock(
    source: &Curve,
    start: f64,
    speed: &Q,
    acceleration: &Q,
    work: &mut Work,
) -> PResult<Curve> {
    let one = Q::one();
    let two = Q::from_integer(2.into());
    let twice_speed = work.op(speed, &two, |a, b| a * b)?;
    let c = work.op(acceleration, &twice_speed, |a, b| a / b)?;
    let old_start = work.lift(source.domain()[0])?;
    let new_start = work.lift(start)?;
    let mut knots = Vec::with_capacity(source.knots.len());
    let mut denominators = Vec::with_capacity(source.knots.len());
    for &knot in &source.knots {
        let knot = work.lift(knot)?;
        let offset = work.op(&knot, &old_start, |a, b| a - b)?;
        let delta = work.op(&c, &offset, |a, b| a * b)?;
        let denominator = work.op(&one, &delta, |a, b| a - b)?;
        if !denominator.is_positive() {
            return Err(Refusal::UnprovedDenominator);
        }
        let weight = work.op(&one, &denominator, |a, b| a / b)?;
        let scaled = work.op(speed, &offset, |a, b| a * b)?;
        let mapped = work.op(&scaled, &weight, |a, b| a * b)?;
        let absolute = work.op(&new_start, &mapped, |a, b| a + b)?;
        knots.push(work.float(&absolute)?);
        denominators.push(weight);
    }
    let factors = basis_factors(&denominators, source.degree, source.weights.len(), work)?;
    reweight_curve(source, ClockBasis { knots, factors }, work)
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
    apply_surface(source, lambda.map(Clock::Factor), max_work)
}

/// Fit independent interior UV landmark correspondences on a tensor
/// surface, retaining the absolute domains and one common weight gauge.
pub fn fit_mobius_spline_surface(
    source: &Surface,
    landmarks: [MobiusLandmark; 2],
    max_work: u64,
) -> Result<Report<Surface>> {
    source.validate()?;
    validate_clock(&[], max_work)?;
    validate_landmark(
        landmarks[0],
        [
            source.knots_u[source.degree_u],
            source.knots_u[source.control_points.len()],
        ],
    )?;
    validate_landmark(
        landmarks[1],
        [
            source.knots_v[source.degree_v],
            source.knots_v[source.control_points[0].len()],
        ],
    )?;
    apply_surface(source, landmarks.map(Clock::Landmark), max_work)
}

fn apply_surface(source: &Surface, clocks: [Clock; 2], max_work: u64) -> Result<Report<Surface>> {
    let nu = source.control_points.len();
    let nv = source.control_points[0].len();
    clamped(source.degree_u, &source.knots_u, nu, source.periodic_u)?;
    clamped(source.degree_v, &source.knots_v, nv, source.periodic_v)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let out = (|| {
        let u = basis(&source.knots_u, source.degree_u, nu, clocks[0], &mut work)?;
        let v = basis(&source.knots_v, source.degree_v, nv, clocks[1], &mut work)?;
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
    clock: Clock,
    w: &mut Work,
) -> PResult<ClockBasis> {
    let a = w.lift(knots[degree])?;
    let b = w.lift(knots[count])?;
    let width = w.op(&b, &a, |a, b| a - b)?;
    let lambda = match clock {
        Clock::Factor(lambda) => w.lift(lambda)?,
        Clock::Landmark(landmark) => factor(landmark, &a, &b, w)?,
    };
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
    Ok(ClockBasis {
        knots: transformed,
        factors: basis_factors(&denominators, degree, count, w)?,
    })
}

fn basis_factors(denominators: &[Q], degree: usize, count: usize, w: &mut Work) -> PResult<Vec<Q>> {
    // Cox–de Boor recurrence: each level adds the denominator at the
    // missing knot. B_i^p(f(t)) = B'_i^p(t) Π D(k'_{i+j}) / D(t)^p.
    let mut factors = Vec::with_capacity(count);
    for i in 0..count {
        let mut product = Q::one();
        for d in &denominators[i + 1..=i + degree] {
            product = w.op(&product, d, |a, b| a * b)?;
        }
        factors.push(product);
    }
    Ok(factors)
}

fn validate_landmark(landmark: MobiusLandmark, [a, b]: [f64; 2]) -> Result<()> {
    check(
        a < landmark.original && landmark.original < b && a < landmark.new && landmark.new < b,
        "Möbius landmarks must both be strictly inside the absolute domain",
    )
}

fn factor(landmark: MobiusLandmark, a: &Q, b: &Q, work: &mut Work) -> PResult<Q> {
    let original = work.lift(landmark.original)?;
    let new = work.lift(landmark.new)?;
    let old_left = work.op(&original, a, |a, b| a - b)?;
    let old_right = work.op(b, &original, |a, b| a - b)?;
    let new_left = work.op(&new, a, |a, b| a - b)?;
    let new_right = work.op(b, &new, |a, b| a - b)?;
    let numerator = work.op(&old_left, &new_right, |a, b| a * b)?;
    let denominator = work.op(&old_right, &new_left, |a, b| a * b)?;
    Ok(work.op(&numerator, &denominator, |a, b| a / b)?)
}
