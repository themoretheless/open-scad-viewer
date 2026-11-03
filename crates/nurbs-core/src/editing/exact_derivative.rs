//! Exact original-clock derivative fields, one rational curve per knot span.
use super::rational_factor::{
    self,
    polynomial::{self as poly, Poly},
};
use crate::{
    Result, check,
    core::bounded_rational::{Error, Work},
    curve::Curve,
};
use num_rational::BigRational as Q;
use num_traits::{One, Zero};
mod extraction;
mod proof;
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    ResourceLimit,
    NotRepresentable,
    OutputDegreeLimit,
    UnprovedDenominator,
    InvalidOutput,
    UnprovedIdentity,
}
impl From<Error> for Refusal {
    fn from(e: Error) -> Self {
        match e {
            Error::ResourceLimit => Self::ResourceLimit,
            Error::NotRepresentable => Self::NotRepresentable,
        }
    }
}
impl From<rational_factor::Refusal> for Refusal {
    fn from(e: rational_factor::Refusal) -> Self {
        match e {
            rational_factor::Refusal::ResourceLimit => Self::ResourceLimit,
            rational_factor::Refusal::NotRepresentable => Self::NotRepresentable,
            rational_factor::Refusal::NonPositiveWeights => Self::UnprovedDenominator,
            rational_factor::Refusal::InvalidOutput => Self::InvalidOutput,
            _ => Self::UnprovedIdentity,
        }
    }
}
type PResult<T> = std::result::Result<T, Refusal>;
#[derive(Clone, Debug)]
pub struct Report {
    /// Complete original-span cover, or None on any refusal. At an internal knot
    /// adjacent pieces own distinct left/right derivative traces; no smoothness
    /// or equality of these traces is inferred. Each piece retains absolute u.
    pub pieces: Option<Vec<Curve>>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}
fn add(a: &Poly, b: &Poly, subtract: bool, w: &mut Work) -> PResult<Poly> {
    let mut out = vec![Q::zero(); a.len().max(b.len())];
    for i in 0..out.len() {
        let zero = Q::zero();
        let x = a.get(i).unwrap_or(&zero);
        let y = b.get(i).unwrap_or(&zero);
        out[i] = w.op(x, y, |x, y| if subtract { x - y } else { x + y })?;
    }
    poly::trim(&mut out);
    Ok(out)
}
fn differentiated(a: &Poly, w: &mut Work) -> PResult<Poly> {
    let mut out = Vec::new();
    for (i, x) in a.iter().enumerate().skip(1) {
        out.push(w.op(x, &Q::from_integer(i.into()), |x, n| x * n)?);
    }
    poly::trim(&mut out);
    Ok(out)
}
fn target(h: &[Poly], width: &Q, w: &mut Work) -> PResult<Vec<Poly>> {
    let denominator = h.last().unwrap();
    let dw = differentiated(denominator, w)?;
    let mut out = Vec::new();
    for a in &h[..h.len() - 1] {
        let da = differentiated(a, w)?;
        let left = poly::multiply(&da, denominator, w)?;
        let right = poly::multiply(a, &dw, w)?;
        let mut n = add(&left, &right, true, w)?;
        for x in &mut n {
            *x = w.op(x, width, |x, width| x / width)?;
        }
        out.push(n);
    }
    out.push(poly::multiply(denominator, denominator, w)?);
    Ok(out)
}
fn reduced(h: &[Poly], w: &mut Work) -> PResult<Vec<Poly>> {
    let mut common = h.last().unwrap().clone();
    for row in &h[..h.len() - 1] {
        common = poly::gcd(common, row.clone(), w)?;
        if common.len() <= 1 {
            break;
        }
    }
    if common.len() <= 1 {
        return Ok(h.to_vec());
    }
    h.iter()
        .map(|row| {
            let (q, r) = poly::divide(row, &common, w)?;
            if !r.is_empty() {
                Err(Refusal::UnprovedIdentity)
            } else {
                Ok(q)
            }
        })
        .collect()
}
fn piece(source: &Curve, span: usize, w: &mut Work) -> PResult<Curve> {
    let a = source.knots[span];
    let b = source.knots[span + 1];
    let qa = w.lift(a)?;
    let qb = w.lift(b)?;
    let width = w.op(&qb, &qa, |a, b| a - b)?;
    let h = extraction::span(source, span, &qa, &width, w)?;
    let target = target(&h, &width, w)?;
    let reduced = reduced(&target, w)?;
    let degree = reduced
        .iter()
        .map(|x| x.len().saturating_sub(1))
        .max()
        .unwrap()
        .max(1);
    if degree > 25 {
        return Err(Refusal::OutputDegreeLimit);
    }
    let channels = reduced
        .iter()
        .map(|row| poly::bernstein(row, degree, w).map_err(Into::into))
        .collect::<PResult<Vec<_>>>()?;
    let (control_points, weights) = super::rational_edit::store(&channels, w)?;
    let out = Curve {
        degree,
        knots: [vec![a; degree + 1], vec![b; degree + 1]].concat(),
        control_points,
        weights,
        periodic: false,
    };
    if out.validate().is_err() {
        return Err(Refusal::InvalidOutput);
    }
    proof::stored(&out, &target, w)?;
    Ok(out)
}
/// Build the vector field dC/du over EVERY original nonempty knot span of a
/// validated 2D/3D curve, including unclamped and periodic storage. Output
/// pieces have clamped nonperiodic storage and the original absolute clock.
/// Their shared boundaries carry one-sided traces, not a fabricated continuous
/// derivative at source knots. Exact binary64 storage or atomic whole-result
/// refusal under the shared 1m-work/4096-bit and unchanged geometry limits.
/// No old source/topology certificate or numerical evaluator-bit transfer.
pub fn curve(source: &Curve, max_work: u64) -> Result<Report> {
    source.validate()?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact derivative work must be in [1,1000000]",
    )?;
    let mut w = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| {
        let mut out = Vec::new();
        for span in source.degree..source.control_points.len() {
            if source.knots[span] < source.knots[span + 1] {
                out.push(piece(source, span, &mut w)?);
            }
        }
        Ok(out)
    })();
    Ok(match result {
        Ok(pieces) => Report {
            pieces: Some(pieces),
            work_used: w.used,
            refusal: None,
        },
        Err(refusal) => Report {
            pieces: None,
            work_used: w.used,
            refusal: Some(refusal),
        },
    })
}
