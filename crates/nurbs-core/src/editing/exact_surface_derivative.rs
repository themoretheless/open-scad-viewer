//! Exact original-clock surface partial fields on every original UV knot cell.
use super::rational_factor::{self, polynomial as uni};
pub use crate::surface::Axis;
use crate::{
    Result, check,
    core::bounded_rational::{Error, Work},
    surface::Surface,
};
use num_rational::BigRational as Q;
use num_traits::{One, Zero};
use std::collections::BTreeMap;
mod content;
mod extraction;
mod mixed;
pub(super) mod polynomial;
pub(super) mod storage;
pub(super) type Poly = BTreeMap<(usize, usize), Q>;
type PResult<T> = std::result::Result<T, Refusal>;
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
#[derive(Clone, Debug)]
pub struct Report {
    /// Complete U-major/V-minor original-cell cover or None on any refusal.
    /// Each nonperiodic clamped patch owns its one-sided boundary traces, without
    /// claiming that adjoining partial derivatives coincide or are regular.
    pub patches: Option<Vec<Surface>>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}
fn target(h: &[Poly], axis: Axis, width: &Q, w: &mut Work) -> PResult<Vec<Poly>> {
    let dw = polynomial::derivative(&h[3], axis, width, w)?;
    let mut out = Vec::new();
    for a in &h[..3] {
        let da = polynomial::derivative(a, axis, width, w)?;
        let left = polynomial::multiply(&da, &h[3], w)?;
        let right = polynomial::multiply(a, &dw, w)?;
        out.push(polynomial::subtract(left, &right, w)?);
    }
    out.push(polynomial::multiply(&h[3], &h[3], w)?);
    Ok(out)
}
#[derive(Clone, Copy)]
pub(super) enum Field {
    Partial(Axis),
    AreaNormal,
    Metric,
    Mixed([usize; 2]),
}
fn area_target(h: &[Poly], widths: &[Q], w: &mut Work) -> PResult<Vec<Poly>> {
    let u = target(h, Axis::U, &widths[0], w)?;
    let v = target(h, Axis::V, &widths[1], w)?;
    let mut out = Vec::new();
    for axis in 0..3 {
        let i = (axis + 1) % 3;
        let j = (axis + 2) % 3;
        let left = polynomial::multiply(&u[i], &v[j], w)?;
        let right = polynomial::multiply(&u[j], &v[i], w)?;
        out.push(polynomial::subtract(left, &right, w)?);
    }
    out.push(polynomial::multiply(&u[3], &v[3], w)?);
    Ok(out)
}
fn metric_target(h: &[Poly], widths: &[Q], w: &mut Work) -> PResult<Vec<Poly>> {
    let u = target(h, Axis::U, &widths[0], w)?;
    let v = target(h, Axis::V, &widths[1], w)?;
    let mut out = Vec::new();
    for (a, b) in [(&u, &u), (&u, &v), (&v, &v)] {
        let mut sum = Poly::new();
        for axis in 0..3 {
            let product = polynomial::multiply(&a[axis], &b[axis], w)?;
            for (key, value) in product {
                polynomial::accumulate(&mut sum, key, &value, w)?;
            }
        }
        out.push(sum);
    }
    out.push(polynomial::multiply(&u[3], &v[3], w)?);
    Ok(out)
}
pub(super) fn original_target(
    s: &Surface,
    ku: usize,
    kv: usize,
    field: Field,
    w: &mut Work,
) -> PResult<Vec<Poly>> {
    let domains = [
        [s.knots_u[ku], s.knots_u[ku + 1]],
        [s.knots_v[kv], s.knots_v[kv + 1]],
    ];
    let mut widths = Vec::new();
    for domain in domains {
        let a = w.lift(domain[0])?;
        let b = w.lift(domain[1])?;
        widths.push(w.op(&b, &a, |a, b| a - b)?);
    }
    let h = extraction::cell(s, ku, kv, domains, &widths, w)?;
    Ok(match field {
        Field::Partial(axis) => target(
            &h,
            axis,
            &widths[match axis {
                Axis::U => 0,
                Axis::V => 1,
            }],
            w,
        )?,
        Field::AreaNormal => area_target(&h, &widths, w)?,
        Field::Metric => metric_target(&h, &widths, w)?,
        Field::Mixed(orders) => mixed::target(&h, orders, &widths, w)?,
    })
}
pub(super) fn reduced_target(target: &[Poly], w: &mut Work) -> PResult<Vec<Poly>> {
    match polynomial::constant_field(target, w)? {
        Some(constant) => Ok(constant),
        None => content::cancel(target, w),
    }
}
fn patch(s: &Surface, ku: usize, kv: usize, field: Field, w: &mut Work) -> PResult<Surface> {
    let domains = [
        [s.knots_u[ku], s.knots_u[ku + 1]],
        [s.knots_v[kv], s.knots_v[kv + 1]],
    ];
    let target = original_target(s, ku, kv, field, w)?;
    let proposal = reduced_target(&target, w)?;
    let out = storage::store(&proposal, domains, w)?;
    storage::prove(&out, &target, w)?;
    Ok(out)
}
/// Construct dS/du or dS/dv over EVERY nonempty original fundamental UV cell.
/// Supports arbitrary validated tensor degrees/knots and periodic encodings.
/// Each output retains absolute U/V clocks. Source boundaries carry one-sided
/// traces; this issues no patch smoothness, normal, trim or solid certificate.
/// One shared1m-work/4096-bit ledger covers all cells and actual-output proofs.
/// Exact positive binary64 storage or atomic whole-result refusal, with the
/// existing degree25/geometry bounds. General bivariate factor cancellation
/// and minimal-degree admission are not claimed.
pub fn surface(source: &Surface, axis: Axis, max_work: u64) -> Result<Report> {
    apply(source, Field::Partial(axis), max_work)
}
/// Construct the oriented NON-UNIT field dS/du cross dS/dv on every original
/// fundamental UV cell. Direction follows the authored parameter order and
/// magnitude is the source area density. Original absolute clocks and one-sided
/// traces are retained. A zero vector is valid geometry, not a regularity proof.
/// No body outward orientation, unit-normal, topology or old certificate credit.
/// Same atomic exact-storage and1m-work/4096-bit/geometry guards as [`surface`].
pub fn area_normal(source: &Surface, max_work: u64) -> Result<Report> {
    apply(source, Field::AreaNormal, max_work)
}
/// Construct the first fundamental coefficients as carrier coordinates
/// [E,F,G]=[Su dot Su,Su dot Sv,Sv dot Sv] on EVERY original UV cell.
/// These are scalar metric fields, not spatial points or normalized tangents.
/// Original clocks/one-sided traces and the same atomic exact-storage guards
/// apply. A singular metric is valid output, not a regularity certificate.
pub fn metric(source: &Surface, max_work: u64) -> Result<Report> {
    apply(source, Field::Metric, max_work)
}
/// Construct d^(a+b)S/du^a dv^b over every original UV cell for total order1..3,
/// matching the existing rational jet ceiling. Higher rational derivatives may
/// remain nonzero even when their order exceeds the source polynomial degree.
/// Original clocks and one-sided traces, not global knot differentiability.
/// One Work ledger and fresh original-source proof cover the complete result.
pub fn mixed_partial(source: &Surface, orders: [usize; 2], max_work: u64) -> Result<Report> {
    check(
        orders[0]
            .checked_add(orders[1])
            .is_some_and(|n| (1..=3).contains(&n)),
        "Mixed derivative total order must be in [1,3]",
    )?;
    apply(source, Field::Mixed(orders), max_work)
}
fn apply(source: &Surface, field: Field, max_work: u64) -> Result<Report> {
    source.validate()?;
    check(
        (1..=cad_predicates::MAX_WORK).contains(&max_work),
        "Exact surface derivative work must be in [1,1000000]",
    )?;
    let mut w = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| {
        let mut out = Vec::new();
        for ku in source.degree_u..source.control_points.len() {
            if source.knots_u[ku] == source.knots_u[ku + 1] {
                continue;
            }
            for kv in source.degree_v..source.control_points[0].len() {
                if source.knots_v[kv] < source.knots_v[kv + 1] {
                    out.push(patch(source, ku, kv, field, &mut w)?);
                }
            }
        }
        Ok(out)
    })();
    Ok(match result {
        Ok(patches) => Report {
            patches: Some(patches),
            work_used: w.used,
            refusal: None,
        },
        Err(refusal) => Report {
            patches: None,
            work_used: w.used,
            refusal: Some(refusal),
        },
    })
}
