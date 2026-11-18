//! Differential-field storage proposals accepted only by continuous error proof.
use super::{
    exact_derivative as exact,
    rational_factor::polynomial::{self as poly, Poly},
};
use crate::{
    Result, check,
    core::bounded_rational::{Error, Work},
    curve::Curve,
};
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};
pub(super) mod proof;
pub(super) mod storage;
type PResult<T> = std::result::Result<T, Refusal>;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Derivative(usize),
    SquaredSpeed,
    Binormal,
    SquaredCurvature,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Refusal {
    ResourceLimit,
    OutputDegreeLimit,
    UnprovedDenominator,
    InvalidOutput,
    ToleranceExceeded,
    UnprovedIdentity,
}
impl From<Error> for Refusal {
    fn from(e: Error) -> Self {
        match e {
            Error::ResourceLimit => Self::ResourceLimit,
            Error::NotRepresentable => Self::InvalidOutput,
        }
    }
}
impl From<exact::Refusal> for Refusal {
    fn from(e: exact::Refusal) -> Self {
        match e {
            exact::Refusal::ResourceLimit => Self::ResourceLimit,
            exact::Refusal::OutputDegreeLimit => Self::OutputDegreeLimit,
            exact::Refusal::UnprovedDenominator => Self::UnprovedDenominator,
            exact::Refusal::UnprovedIdentity => Self::UnprovedIdentity,
            _ => Self::InvalidOutput,
        }
    }
}
impl From<super::rational_factor::Refusal> for Refusal {
    fn from(e: super::rational_factor::Refusal) -> Self {
        Self::from(exact::Refusal::from(e))
    }
}
#[derive(Clone, Debug)]
pub struct Report {
    /// Every original nonempty span, or no output at all on any refusal.
    pub pieces: Option<Vec<Curve>>,
    /// Euclidean field-value error upper bounds, one per piece, inclusive of
    /// all stored control and weight rounding. Not sampled evaluator residuals.
    pub error_upper: Option<Vec<f64>>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}
fn original(field: Field, dim: usize) -> Result<exact::Field> {
    Ok(match field {
        Field::Derivative(n) => {
            check((1..=3).contains(&n), "Derivative order must be in [1,3]")?;
            exact::Field::Derivative(n)
        }
        Field::SquaredSpeed => exact::Field::SquaredSpeed,
        Field::Binormal => {
            check(dim == 3, "Binormal field requires a 3D curve")?;
            exact::Field::Binormal
        }
        Field::SquaredCurvature => exact::Field::SquaredCurvature,
    })
}
fn piece(
    source: &Curve,
    span: usize,
    field: exact::Field,
    tolerance: &Q,
    max_error: f64,
    w: &mut Work,
) -> PResult<(Curve, f64)> {
    let raw = exact::original_target(source, span, field, w)?;
    let mut target = exact::reduced(&raw, w)?;
    let degree = target
        .iter()
        .map(|p| p.len().saturating_sub(1))
        .max()
        .unwrap()
        .max(1);
    if degree > 25 {
        return Err(Refusal::OutputDegreeLimit);
    }
    let mut channels = target
        .iter()
        .map(|p| poly::bernstein(p, degree, w).map_err(Refusal::from))
        .collect::<PResult<Vec<_>>>()?;
    if channels.last().unwrap().iter().all(Signed::is_negative) {
        for row in target.iter_mut().chain(channels.iter_mut()) {
            for x in row {
                *x = w.op(x, &Q::zero(), |a, _| -a)?;
            }
        }
    }
    if !channels.last().unwrap().iter().all(Signed::is_positive) {
        return Err(Refusal::UnprovedDenominator);
    }
    // Re-prove the reduced target against the uncancelled original expression.
    // Original denominators were admitted by source weights/curvature regularity.
    for axis in 0..target.len() - 1 {
        let a = poly::multiply(&target[axis], raw.last().unwrap(), w)?;
        let b = poly::multiply(&raw[axis], target.last().unwrap(), w)?;
        if a != b {
            return Err(Refusal::UnprovedIdentity);
        }
    }
    let (control_points, weights) = storage::candidate(&channels, w)?;
    let out = Curve {
        degree,
        knots: [
            vec![source.knots[span]; degree + 1],
            vec![source.knots[span + 1]; degree + 1],
        ]
        .concat(),
        control_points,
        weights,
        periodic: false,
    };
    if out.validate().is_err() {
        return Err(Refusal::InvalidOutput);
    }
    let squared = proof::squared_error(&out, &target, channels.last().unwrap(), w)?;
    if &squared > tolerance {
        return Err(Refusal::ToleranceExceeded);
    }
    let upper = proof::ceil_sqrt(&squared, max_error, w)?;
    Ok((out, upper))
}
/// Construct an original-clock derivative (order1..3), squared speed,
/// unnormalized spatial binormal or squared curvature field. Exact storage is
/// tried first; otherwise binary64 proposals require a fresh continuous
/// Euclidean field-value error bound <= max_error. No adaptive fitting or
/// degree reduction. Original-source curvature regularity is required before
/// cancellation. Output degree25, shared1m-work/4096-bit and geometry limits
/// remain unchanged. Any late refusal discards every piece and bound.
pub fn curve(source: &Curve, field: Field, max_error: f64, max_work: u64) -> Result<Report> {
    source.validate()?;
    let field = original(field, source.control_points[0].len())?;
    apply(source, field, max_error, max_work)
}
/// Construct the spatial torsion carrier with a continuous field-value error
/// bound. Original nonzero binormal admission precedes cancellation. Existing
/// exact-storage, positive-denominator, work/bit and geometry guards apply.
pub fn torsion(source: &Curve, max_error: f64, max_work: u64) -> Result<Report> {
    source.validate()?;
    check(
        source.control_points[0].len() == 3,
        "Torsion field requires a 3D curve",
    )?;
    apply(source, exact::Field::Torsion, max_error, max_work)
}
fn apply(source: &Curve, field: exact::Field, max_error: f64, max_work: u64) -> Result<Report> {
    check(
        max_error.is_finite() && max_error > 0.,
        "Field error must be finite and positive",
    )?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Field work must be in [1,1000000]",
    )?;
    let mut w = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| {
        let tol = w.lift(max_error)?;
        let tolerance = w.op(&tol, &tol, |a, b| a * b)?;
        let mut pieces = Vec::new();
        let mut bounds = Vec::new();
        for span in source.degree..source.control_points.len() {
            if source.knots[span] < source.knots[span + 1] {
                let (p, b) = piece(source, span, field, &tolerance, max_error, &mut w)?;
                pieces.push(p);
                bounds.push(b);
            }
        }
        Ok((pieces, bounds))
    })();
    Ok(match result {
        Ok((pieces, bounds)) => Report {
            pieces: Some(pieces),
            error_upper: Some(bounds),
            work_used: w.used,
            refusal: None,
        },
        Err(refusal) => Report {
            pieces: None,
            error_upper: None,
            work_used: w.used,
            refusal: Some(refusal),
        },
    })
}
