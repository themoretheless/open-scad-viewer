//! Bounded translation proposal with a common exact amplitude per coordinate.
//! This supplies NO trim ownership, rank, smoothness or Solid admission.
use crate::{LoftCap, Result, invalid};
use num_bigint::BigInt;
use num_rational::BigRational as Q;
use num_traits::{One, Signed, ToPrimitive, Zero};
use nurbs_core::interval_eval::Interval as I;

pub struct Report {
    pub cap: Option<LoftCap>,
    /// Whole corresponding filled carrier, from unchanged positive weights.
    pub displacement_upper: Option<f64>,
    pub modified_axes: [bool; 3],
    pub work: u64,
    pub reason: &'static str,
}
fn step(out: &mut Report, max: u64) -> bool {
    if out.work == max {
        out.reason = "rational-cap-translation-work-limit";
        false
    } else {
        out.work += 1;
        true
    }
}
fn exact(q: &Q) -> Option<f64> {
    let f = q.to_f64()?;
    (f.is_finite() && Q::from_float(f).as_ref() == Some(q)).then_some(f)
}
fn axis(
    values: &[f64],
    anchor: f64,
    translation: f64,
    out: &mut Report,
    max: u64,
) -> Option<(Vec<f64>, bool)> {
    let values = values
        .iter()
        .map(|v| Q::from_float(*v))
        .collect::<Option<Vec<_>>>()?;
    axis_exact(
        &values,
        &Q::from_float(anchor)?,
        &Q::from_float(translation)?,
        out,
        max,
    )
}
pub(crate) fn axis_exact(
    values: &[Q],
    a: &Q,
    t: &Q,
    out: &mut Report,
    max: u64,
) -> Option<(Vec<f64>, bool)> {
    let base = a + t;
    let mut offsets = Vec::with_capacity(values.len());
    let mut literal = Vec::with_capacity(values.len());
    for q in values {
        if !step(out, max) {
            return None;
        }
        if let Some(f) = exact(&(q + t)) {
            literal.push(f);
        }
        offsets.push(q - a);
    }
    if literal.len() == values.len() {
        return Some((literal, false));
    }
    // Dyadic input denominators are nested powers of two. Their maximum is
    // the exact common denominator; no floating gcd or rank decision enters.
    let denominator = offsets.iter().map(|q| q.denom()).max()?.clone();
    let integers: Vec<BigInt> = offsets
        .iter()
        .map(|q| q.numer() * (&denominator / q.denom()))
        .collect();
    let mut gcd = BigInt::zero();
    for z in &integers {
        let mut b = z.abs();
        while !b.is_zero() {
            if !step(out, max) {
                return None;
            }
            let r = &gcd % &b;
            gcd = b;
            b = r;
        }
    }
    if gcd.is_zero() {
        out.reason = "rational-cap-translation-anchor-unrepresentable";
        return None;
    }
    let mut magnitude: f64 = 0.;
    for q in values {
        let f = (q + t).to_f64()?;
        if !f.is_finite() {
            out.reason = "rational-cap-translation-grid-unrepresentable";
            return None;
        }
        magnitude = magnitude.max(f.abs().next_up());
    }
    // A common binary grid at the upper magnitude mantissa spacing.
    // The final rational equality check, rather than this proposal, decides
    // whether every translated coordinate is exactly representable.
    let exponent = ((magnitude.to_bits() >> 52) & 2047) as i32 - 1023 - 52;
    if !(-1022..=970).contains(&exponent) {
        out.reason = "rational-cap-translation-grid-unrepresentable";
        return None;
    }
    let quantum = Q::from_float(2f64.powi(exponent))?;
    let units = quantum * Q::from_integer(denominator.clone());
    let grid = if units < Q::one() {
        BigInt::one()
    } else {
        if units.denom() != &BigInt::one() {
            return None;
        }
        units.to_integer()
    };
    let nearest: BigInt = ((&gcd * 2u32 + &grid) / (&grid * 2u32)) * &grid;
    // Choose the nearest POSITIVE amplitude. The full pole-error bound below
    // must still fit the caller's unchanged tolerance; never flatten a source.
    let rounded = nearest.max(grid.clone());
    let mut result = Vec::with_capacity(values.len());
    for z in integers {
        if !step(out, max) {
            return None;
        }
        let value = &base + Q::new((z / &gcd) * &rounded, denominator.clone());
        let Some(f) = exact(&value) else {
            out.reason = "rational-cap-translation-coefficient-unrepresentable";
            return None;
        };
        result.push(f);
    }
    Some((result, rounded != gcd))
}
/// Propose translation while retaining the complete rational tensor and trims.
/// A common coordinate amplitude can change; all source boundary equalities
/// MUST be freshly checked against the actual translated walls by the caller.
/// `anchor` is a proposal origin, not a caller-supplied geometric certificate.
/// Fixed positive weights transfer the pole displacement bound to every UV.
pub fn propose(
    cap: &LoftCap,
    translation: [f64; 3],
    anchor: [f64; 3],
    tolerance: f64,
    max_work: u64,
) -> Result<Report> {
    cap.surface.validate()?;
    let s = &cap.surface;
    if max_work > 1000000
        || !tolerance.is_finite()
        || tolerance < 0.
        || translation.iter().chain(&anchor).any(|x| !x.is_finite())
        || s.control_points.iter().flatten().any(|p| p.len() != 3)
        || s.control_points
            .len()
            .saturating_mul(s.control_points[0].len())
            > 4096
    {
        return Err(invalid("Invalid rational cap translation proposal"));
    }
    let mut out = Report {
        cap: None,
        displacement_upper: None,
        modified_axes: [false; 3],
        work: 0,
        reason: "rational-cap-translation-unproved",
    };
    let mut candidate = cap.clone();
    for k in 0..3 {
        let values: Vec<f64> = s.control_points.iter().flatten().map(|p| p[k]).collect();
        let Some((coordinates, modified)) =
            axis(&values, anchor[k], translation[k], &mut out, max_work)
        else {
            return Ok(out);
        };
        for (p, v) in candidate
            .surface
            .control_points
            .iter_mut()
            .flatten()
            .zip(coordinates)
        {
            p[k] = v;
        }
        out.modified_axes[k] = modified;
    }
    let mut bound: f64 = 0.;
    for (source, target) in s
        .control_points
        .iter()
        .flatten()
        .zip(candidate.surface.control_points.iter().flatten())
    {
        if !step(&mut out, max_work) {
            return Ok(out);
        }
        if !out.modified_axes.iter().any(|v| *v) {
            continue;
        }
        let mut squared = I::point(0.);
        for k in 0..3 {
            if !out.modified_axes[k] {
                continue;
            }
            let d = I::point(target[k]).sub(I::point(source[k]).add(I::point(translation[k]))?)?;
            squared = squared.add(d.mul(d)?)?;
        }
        bound = bound.max(squared.hi.max(0.).sqrt().next_up());
    }
    out.displacement_upper = Some(bound);
    if !bound.is_finite() || bound > tolerance {
        out.reason = "rational-cap-translation-error-budget-exceeded";
        return Ok(out);
    }
    out.cap = Some(candidate);
    out.reason = "rational-cap-translation-proposal";
    Ok(out)
}
