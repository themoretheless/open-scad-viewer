//! Exact rational curve refinement, restriction, and binary64 storage.
use crate::{Result, curve::Curve};
use num_bigint::BigInt;
use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};
type PResult<T> = std::result::Result<T, Refusal>;
use crate::core::bounded_rational::Work;
pub(crate) fn integer_gcd(mut a: BigInt, mut b: BigInt, w: &mut Work) -> PResult<BigInt> {
    while !b.is_zero() {
        let r = w.int(&a, &b, |a, b| a % b)?;
        a = b;
        b = r;
    }
    Ok(a)
}

pub(crate) fn store(b: &[Vec<Q>], w: &mut Work) -> PResult<(Vec<Vec<f64>>, Vec<f64>)> {
    let dim = b.len() - 1;
    let weights = &b[dim];
    if !(weights.iter().all(|x| x.is_positive()) || weights.iter().all(|x| x.is_negative())) {
        return Err(Refusal::NonPositiveWeights);
    }
    // A single global rational gauge: clear weight denominators, remove the
    // common integer gcd, then choose ONE dyadic scale for the complete curve.
    let mut lcm = BigInt::one();
    for x in weights {
        let g = integer_gcd(lcm.clone(), x.denom().clone(), w)?;
        let a = w.int(&lcm, &g, |a, b| a / b)?;
        lcm = w.int(&a, x.denom(), |a, b| a * b)?;
    }
    let mut integers = Vec::new();
    let mut common = BigInt::zero();
    for x in weights {
        let ratio = w.int(&lcm, x.denom(), |a, b| a / b)?;
        let a = w.int(&x.numer().abs(), &ratio, |a, b| a * b)?;
        common = integer_gcd(common, a.clone(), w)?;
        integers.push(a);
    }
    for a in &mut integers {
        *a = w.int(a, &common, |a, b| a / b)?;
    }
    let shift = integers
        .iter()
        .map(BigInt::bits)
        .max()
        .ok_or(Refusal::InvalidOutput)?
        .saturating_sub(1);
    w.charge(1 + shift / 64)?;
    let denominator = Q::from_integer(BigInt::one() << shift as usize);
    let mut controls = Vec::new();
    let mut output_weights = Vec::new();
    for (i, integer) in integers.into_iter().enumerate() {
        let mut cp = Vec::new();
        for row in &b[..dim] {
            let x = w.op(&row[i], &weights[i], |a, b| a / b)?;
            cp.push(w.float(&x)?);
        }
        controls.push(cp);
        let normalized = w.op(&Q::from_integer(integer), &denominator, |a, b| a / b)?;
        output_weights.push(w.float(&normalized)?);
    }
    Ok((controls, output_weights))
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    NoCommonFactor,
    ResourceLimit,
    NotRepresentable,
    NonPositiveWeights,
    InvalidOutput,
    UnprovedFactor,
    DifferentDefinition,
    PredicateIndeterminate(cad_predicates::Reason),
}
impl From<crate::core::bounded_rational::Error> for Refusal {
    fn from(error: crate::core::bounded_rational::Error) -> Self {
        match error {
            crate::core::bounded_rational::Error::ResourceLimit => Self::ResourceLimit,
            crate::core::bounded_rational::Error::NotRepresentable => Self::NotRepresentable,
        }
    }
}
pub(crate) fn homogeneous(
    source: &Curve,
    parameter: f64,
    count: usize,
    w: &mut Work,
) -> PResult<(Vec<Vec<Q>>, Vec<f64>)> {
    let p = source.degree;
    let dim = source.control_points[0].len();
    let mut knots = source.knots.clone();
    let mut net = Vec::with_capacity(source.control_points.len());
    for (cp, &weight) in source.control_points.iter().zip(&source.weights) {
        let weight = w.lift(weight)?;
        let mut row = Vec::with_capacity(dim + 1);
        for &x in cp {
            let x = w.lift(x)?;
            row.push(w.op(&x, &weight, |a, b| a * b)?);
        }
        row.push(weight);
        net.push(row);
    }
    refine_homogeneous(&mut net, &mut knots, p, parameter, count, w)?;
    Ok((net, knots))
}

pub(crate) fn refine_homogeneous(
    net: &mut Vec<Vec<Q>>,
    knots: &mut Vec<f64>,
    p: usize,
    parameter: f64,
    count: usize,
    w: &mut Work,
) -> PResult<()> {
    let dim = net[0].len() - 1;
    let u = w.lift(parameter)?;
    for _ in 0..count {
        let k = knots.partition_point(|x| *x <= parameter) - 1;
        let s = knots.iter().filter(|x| **x == parameter).count();
        let n = net.len();
        let mut refined = vec![vec![Q::zero(); dim + 1]; n + 1];
        refined[..=k - p].clone_from_slice(&net[..=k - p]);
        refined[k - s + 1..].clone_from_slice(&net[k - s..]);
        for i in k - p + 1..=k - s {
            let left = w.lift(knots[i])?;
            let right = w.lift(knots[i + p])?;
            let denominator = w.op(&right, &left, |a, b| a - b)?;
            if denominator <= Q::zero() {
                return Err(Refusal::InvalidOutput);
            }
            let distance = w.op(&u, &left, |a, b| a - b)?;
            let alpha = w.op(&distance, &denominator, |a, b| a / b)?;
            if alpha < Q::zero() || alpha > Q::one() {
                return Err(Refusal::InvalidOutput);
            }
            let beta = w.op(&Q::one(), &alpha, |a, b| a - b)?;
            for axis in 0..=dim {
                let a = w.op(&alpha, &net[i][axis], |a, b| a * b)?;
                let b = w.op(&beta, &net[i - 1][axis], |a, b| a * b)?;
                refined[i][axis] = w.op(&a, &b, |a, b| a + b)?;
            }
        }
        knots.insert(k + 1, parameter);
        *net = refined;
    }
    Ok(())
}
pub(crate) struct ClampReport {
    pub(crate) curve: Option<Curve>,
    pub(crate) work_used: u64,
    pub(crate) refusal: Option<Refusal>,
}

/// Represent an original nonperiodic interval with clamped endpoint knots.
pub(crate) fn clamp_active(
    source: &Curve,
    range: Option<[f64; 2]>,
    max_work: u64,
) -> Result<ClampReport> {
    source.validate()?;
    crate::check(
        !source.periodic && (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact active clamping needs a nonperiodic curve and bounded work",
    )?;
    let [start, end] = source.domain();
    let range = range.unwrap_or([start, end]);
    crate::check(
        start <= range[0] && range[0] < range[1] && range[1] <= end,
        "Exact active clamping needs an increasing interval within the original domain",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = work
        .charge((source.control_points.len() * (2 * source.degree + 1)) as u64)
        .map_err(Refusal::from)
        .and_then(|()| clamp_curve_interval(source, range, &mut work));
    let (curve, refusal) = match result {
        Ok(curve) => (Some(curve), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(ClampReport {
        curve,
        work_used: work.used,
        refusal,
    })
}

pub(crate) fn clamp_curve_interval(
    source: &Curve,
    [lo, hi]: [f64; 2],
    work: &mut Work,
) -> std::result::Result<Curve, Refusal> {
    let degree = source.degree;
    let (active, clamped_knots) = clamp_curve_net(source, [lo, hi], work)?;
    let channels: Vec<Vec<_>> = (0..active[0].len())
        .map(|axis| active.iter().map(|row| row[axis].clone()).collect())
        .collect();
    let (control_points, weights) = store(&channels, work)?;
    let curve = Curve {
        degree,
        knots: clamped_knots,
        control_points,
        weights,
        periodic: false,
    };
    curve.validate().map_err(|_| Refusal::InvalidOutput)?;
    Ok(curve)
}

pub(crate) fn clamp_curve_net(
    source: &Curve,
    [lo, hi]: [f64; 2],
    work: &mut Work,
) -> std::result::Result<(Vec<Vec<num_rational::BigRational>>, Vec<f64>), Refusal> {
    let degree = source.degree;
    let (mut net, mut knots) = homogeneous(source, lo, 0, work)?;
    for end in [lo, hi] {
        let multiplicity = knots.iter().filter(|&&knot| knot == end).count();
        if multiplicity < degree {
            refine_homogeneous(
                &mut net,
                &mut knots,
                degree,
                end,
                degree - multiplicity,
                work,
            )?;
        }
    }
    let after_start = knots.partition_point(|&k| k <= lo);
    let before_end = knots.partition_point(|&k| k < hi);
    // The first active span starts at the last copy of lo. The last
    // active span ends at the first copy of hi. This also handles source
    // endpoints that are already clamped at degree plus one.
    let active = net.drain(after_start - degree - 1..before_end).collect();
    let mut clamped_knots = vec![lo; degree + 1];
    clamped_knots.extend_from_slice(&knots[after_start..before_end]);
    clamped_knots.extend(std::iter::repeat_n(hi, degree + 1));
    Ok((active, clamped_knots))
}
