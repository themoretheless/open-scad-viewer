//! Cancel common polynomial factors, with complete original-clock identity.
use crate::{Result, check, core::curve_identity::compare_definitions, curve::Curve};
use num_bigint::BigInt;
use num_rational::BigRational as Q;
use num_traits::{One, Signed, ToPrimitive, Zero};
mod polynomial;
use polynomial::{Poly, Work};
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
#[derive(Clone, Debug)]
pub struct Report {
    /// Literal source on every refusal, never a partially simplified curve.
    pub curve: Curve,
    pub simplified: bool,
    pub factor_degree: usize,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}
type PResult<T> = std::result::Result<T, Refusal>;
fn quotients(s: &Curve, w: &mut Work) -> PResult<(usize, Vec<Poly>)> {
    let dim = s.control_points[0].len();
    let mut h = vec![Vec::new(); dim + 1];
    for (cp, &weight) in s.control_points.iter().zip(&s.weights) {
        let q = w.lift(weight)?;
        for axis in 0..dim {
            let x = w.lift(cp[axis])?;
            h[axis].push(w.op(&x, &q, |a, b| a * b)?);
        }
        h[dim].push(q);
    }
    let powers = h
        .iter()
        .map(|row| polynomial::power(row, w))
        .collect::<PResult<Vec<_>>>()?;
    let mut g = powers[dim].clone();
    for n in &powers[..dim] {
        g = polynomial::gcd(g, n.clone(), w)?;
        if g.len() == 1 {
            return Err(Refusal::NoCommonFactor);
        }
    }
    let factor = g.len().saturating_sub(1);
    if factor == 0 {
        return Err(Refusal::NoCommonFactor);
    }
    let mut out = Vec::new();
    for original in &powers {
        let (q, r) = polynomial::divide(original, &g, w)?;
        if !r.is_empty() || polynomial::multiply(&g, &q, w)? != *original {
            return Err(Refusal::UnprovedFactor);
        }
        out.push(q);
    }
    Ok((factor, out))
}
fn candidate(source: &Curve, h: &[Poly], degree: usize, w: &mut Work) -> PResult<Curve> {
    let b = h
        .iter()
        .map(|row| polynomial::bernstein(row, degree, w))
        .collect::<PResult<Vec<_>>>()?;
    let dim = b.len() - 1;
    let weights = &b[dim];
    if !(weights.iter().all(|x| x.is_positive()) || weights.iter().all(|x| x.is_negative())) {
        return Err(Refusal::NonPositiveWeights);
    }
    // A single global rational gauge: clear weight denominators, remove the
    // common integer gcd, then choose ONE dyadic scale for the complete curve.
    let mut lcm = BigInt::one();
    for x in weights {
        let g = polynomial::integer_gcd(lcm.clone(), x.denom().clone(), w)?;
        let a = w.int(&lcm, &g, |a, b| a / b)?;
        lcm = w.int(&a, x.denom(), |a, b| a * b)?;
    }
    let mut integers = Vec::new();
    let mut common = BigInt::zero();
    for x in weights {
        let ratio = w.int(&lcm, x.denom(), |a, b| a / b)?;
        let a = w.int(&x.numer().abs(), &ratio, |a, b| a * b)?;
        common = polynomial::integer_gcd(common, a.clone(), w)?;
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
    let domain = source.domain();
    let curve = Curve {
        degree,
        knots: [vec![domain[0]; degree + 1], vec![domain[1]; degree + 1]].concat(),
        control_points: controls,
        weights: output_weights,
        periodic: false,
    };
    if curve.validate().is_err() {
        return Err(Refusal::InvalidOutput);
    }
    Ok(curve)
}
/// Exact common-factor cancellation for one nonperiodic clamped rational
/// Bezier segment (existing degree1..25 and 2D/3D limits). A constant primitive
/// uses valid degree1 storage. Full original parameter domain is retained.
/// A bounded rational proposal is independently verified against actual output
/// binary64 definitions. Resource/refusal returns the unchanged source.
pub fn cancel(source: &Curve, max_work: u64) -> Result<Report> {
    source.validate()?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Rational factor work must be in1..=1000000",
    )?;
    let p = source.degree;
    let domain = source.domain();
    check(
        !source.periodic
            && source.control_points.len() == p + 1
            && source.knots[..=p].iter().all(|&x| x == domain[0])
            && source.knots[p + 1..].iter().all(|&x| x == domain[1]),
        "Rational factor cancellation requires one nonperiodic clamped Bezier segment",
    )?;
    let mut w = Work {
        used: 0,
        limit: max_work,
    };
    let refused = |reason, work_used, factor_degree| Report {
        curve: source.clone(),
        simplified: false,
        refusal: Some(reason),
        work_used,
        factor_degree,
    };
    let (factor, h) = match quotients(source, &mut w) {
        Ok(q) => q,
        Err(r) => return Ok(refused(r, w.used, 0)),
    };
    let minimum = h
        .iter()
        .map(|p| p.len().saturating_sub(1))
        .max()
        .unwrap_or(0)
        .max(1);
    let maximum = p.saturating_sub(1).max(1);
    let mut last = Refusal::NotRepresentable;
    for degree in minimum..=maximum {
        let proposed = match candidate(source, &h, degree, &mut w) {
            Ok(c) => c,
            Err(Refusal::ResourceLimit) => {
                return Ok(refused(Refusal::ResourceLimit, w.used, factor));
            }
            Err(r) => {
                last = r;
                continue;
            }
        };
        if w.used == w.limit {
            return Ok(refused(Refusal::ResourceLimit, w.used, factor));
        }
        let proof = compare_definitions(source, &proposed, 0., w.limit - w.used, false)?;
        w.used += proof.work_used;
        match proof.outcome {
            cad_predicates::BezierIdentity::Equal => {
                return Ok(Report {
                    curve: proposed,
                    simplified: true,
                    factor_degree: factor,
                    work_used: w.used,
                    refusal: None,
                });
            }
            cad_predicates::BezierIdentity::Different => {
                return Ok(refused(Refusal::DifferentDefinition, w.used, factor));
            }
            cad_predicates::BezierIdentity::Indeterminate(
                cad_predicates::Reason::ResourceLimit,
            ) => return Ok(refused(Refusal::ResourceLimit, w.used, factor)),
            cad_predicates::BezierIdentity::Indeterminate(r) => {
                return Ok(refused(Refusal::PredicateIndeterminate(r), w.used, factor));
            }
        }
    }
    Ok(refused(last, w.used, factor))
}
