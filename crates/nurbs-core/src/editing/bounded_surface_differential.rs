//! Continuous whole-cell error admission for stored surface differential fields.
use super::{bounded_differential as bounded, exact_surface_derivative as exact};
use crate::{Result, check, core::bounded_rational::Work, surface::Surface};
pub use bounded::Refusal;
pub use exact::Axis;
use exact::{Poly, polynomial as poly, storage};
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};
mod proof;
type PResult<T> = std::result::Result<T, Refusal>;
#[derive(Clone, Copy)]
pub enum Field {
    Partial(Axis),
    Mixed([usize; 2]),
    AreaNormal,
    Metric,
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
#[derive(Clone, Debug)]
pub struct Report {
    /// U-major/V-minor complete fundamental-cell cover, or no output on refusal.
    pub patches: Option<Vec<Surface>>,
    /// Continuous Euclidean error in the three field carrier coordinates.
    /// For metric [E,F,G], this is a numeric carrier norm, not an intrinsic tensor norm.
    pub error_upper: Option<Vec<f64>>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}
fn original(f: Field) -> Result<exact::Field> {
    Ok(match f {
        Field::Partial(a) => exact::Field::Partial(a),
        Field::AreaNormal => exact::Field::AreaNormal,
        Field::Metric => exact::Field::Metric,
        Field::Mixed(orders) => {
            check(
                orders[0]
                    .checked_add(orders[1])
                    .is_some_and(|n| (1..=3).contains(&n)),
                "Mixed derivative total order must be in [1,3]",
            )?;
            exact::Field::Mixed(orders)
        }
    })
}
fn patch(
    s: &Surface,
    ku: usize,
    kv: usize,
    field: exact::Field,
    tolerance: &Q,
    max_error: f64,
    w: &mut Work,
) -> PResult<(Surface, f64)> {
    let raw = exact::original_target(s, ku, kv, field, w)?;
    let mut target = exact::reduced_target(&raw, w)?;
    let (pu, pv) = storage::degrees(&target);
    if pu > 25 || pv > 25 {
        return Err(Refusal::OutputDegreeLimit);
    }
    let mut channels = target
        .iter()
        .map(|p| storage::bernstein(p, pu, pv, w).map_err(Refusal::from))
        .collect::<PResult<Vec<_>>>()?;
    if channels[3].iter().all(Signed::is_negative) {
        for row in &mut target {
            for x in row.values_mut() {
                *x = w.op(x, &Q::zero(), |a, _| -a)?;
            }
        }
        for row in &mut channels {
            for x in row {
                *x = w.op(x, &Q::zero(), |a, _| -a)?;
            }
        }
    }
    if !channels[3].iter().all(Signed::is_positive) {
        return Err(Refusal::UnprovedDenominator);
    }
    for axis in 0..3 {
        let a = poly::multiply(&target[axis], &raw[3], w)?;
        let b = poly::multiply(&raw[axis], &target[3], w)?;
        if a != b {
            return Err(Refusal::UnprovedIdentity);
        }
    }
    let (points, weights) = bounded::storage::candidate(&channels, w)?;
    let domains = [
        [s.knots_u[ku], s.knots_u[ku + 1]],
        [s.knots_v[kv], s.knots_v[kv + 1]],
    ];
    let out = Surface {
        degree_u: pu,
        degree_v: pv,
        knots_u: [vec![domains[0][0]; pu + 1], vec![domains[0][1]; pu + 1]].concat(),
        knots_v: [vec![domains[1][0]; pv + 1], vec![domains[1][1]; pv + 1]].concat(),
        control_points: points.chunks(pv + 1).map(|r| r.to_vec()).collect(),
        weights: weights.chunks(pv + 1).map(|r| r.to_vec()).collect(),
        periodic_u: false,
        periodic_v: false,
    };
    if out.validate().is_err() {
        return Err(Refusal::InvalidOutput);
    }
    let squared = proof::squared_error(&out, &target, &channels[3], w)?;
    if &squared > tolerance {
        return Err(Refusal::ToleranceExceeded);
    }
    let upper = bounded::proof::ceil_sqrt(&squared, max_error, w)?;
    Ok((out, upper))
}
/// Build partial/mixed order1..3, non-unit area normal or [E,F,G] metric fields
/// with a continuous error <= max_error on every original UV cell. Original
/// clocks and one-sided traces; no adaptive fitting, invariant/sign preservation,
/// unit-normal, regularity, topology or evaluator-bit authority. Output degrees25,
/// geometry/weight limits and the shared1m-work/4096-bit ledger are unchanged.
/// Any late refusal discards every earlier patch and bound.
pub fn surface(source: &Surface, field: Field, max_error: f64, max_work: u64) -> Result<Report> {
    source.validate()?;
    let field = original(field)?;
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
        let t = w.lift(max_error)?;
        let tolerance = w.op(&t, &t, |a, b| a * b)?;
        let mut patches = Vec::new();
        let mut bounds = Vec::new();
        for ku in source.degree_u..source.control_points.len() {
            if source.knots_u[ku] == source.knots_u[ku + 1] {
                continue;
            }
            for kv in source.degree_v..source.control_points[0].len() {
                if source.knots_v[kv] < source.knots_v[kv + 1] {
                    let (p, b) = patch(source, ku, kv, field, &tolerance, max_error, &mut w)?;
                    patches.push(p);
                    bounds.push(b);
                }
            }
        }
        Ok((patches, bounds))
    })();
    Ok(match result {
        Ok((patches, bounds)) => Report {
            patches: Some(patches),
            error_upper: Some(bounds),
            work_used: w.used,
            refusal: None,
        },
        Err(refusal) => Report {
            patches: None,
            error_upper: None,
            work_used: w.used,
            refusal: Some(refusal),
        },
    })
}
