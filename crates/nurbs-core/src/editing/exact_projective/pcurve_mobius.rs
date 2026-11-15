//! Pull back retained UV curves under independent Möbius chart clocks.
use super::mobius::validate_clock;
use crate::editing::{
    exact_derivative::{self as exact, Refusal, extraction, proof},
    rational_edit,
    rational_factor::polynomial::{self as poly, Poly},
};
use crate::{Result, check, core::bounded_rational::Work, curve::Curve};
use num_rational::BigRational as Q;
use num_traits::{One, Signed};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PcurvePullbackRefusal {
    Geometry(Refusal),
    /// A complete original-span proof of membership in the chart was unavailable.
    UnprovedChartDomain,
}

impl From<Refusal> for PcurvePullbackRefusal {
    fn from(reason: Refusal) -> Self {
        Self::Geometry(reason)
    }
}

#[derive(Clone, Debug)]
pub struct PcurvePullback {
    /// Complete cover of the original pcurve parameter domain, in order.
    /// At a knot, neighboring pieces retain their original one-sided traces.
    pub pieces: Option<Vec<Curve>>,
    pub work_used: u64,
    pub refusal: Option<PcurvePullbackRefusal>,
}

/// Transfer a retained two-dimensional pcurve from the original UV chart
/// to its independently reclocked chart. The provided factors describe
/// the map from new UV to old UV. The pcurve's own absolute parameter is
/// preserved, and every new piece is verified against the exact inverse
/// map over an entire original span. An uncertain chart or storage refusal
/// withholds all pieces.
pub fn pullback_mobius_pcurve(
    source: &Curve,
    domain: [[f64; 2]; 2],
    lambda: [f64; 2],
    max_work: u64,
) -> Result<PcurvePullback> {
    source.validate()?;
    check(
        source.control_points[0].len() == 2,
        "A retained pcurve needs exactly two coordinates",
    )?;
    check(
        domain
            .iter()
            .all(|[a, b]| a.is_finite() && b.is_finite() && a < b),
        "Pcurve pullback needs two finite nonempty UV intervals",
    )?;
    validate_clock(&lambda, max_work)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let (pieces, refusal) = match pieces(source, domain, lambda, &mut work) {
        Ok(pieces) => (Some(pieces), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(PcurvePullback {
        pieces,
        refusal,
        work_used: work.used,
    })
}

fn pieces(
    source: &Curve,
    domain: [[f64; 2]; 2],
    lambda: [f64; 2],
    work: &mut Work,
) -> std::result::Result<Vec<Curve>, PcurvePullbackRefusal> {
    let axes = [
        inverse_axis(domain[0], lambda[0], work)?,
        inverse_axis(domain[1], lambda[1], work)?,
    ];
    let mut pieces = Vec::new();
    for span in source.degree..source.control_points.len() {
        let [a, b] = [source.knots[span], source.knots[span + 1]];
        if a == b {
            continue;
        }
        let h = original_span(source, span, [a, b], work)?;
        let u = remap_component(&h[0], &h[2], &axes[0], source.degree, work)?;
        let v = remap_component(&h[1], &h[2], &axes[1], source.degree, work)?;
        pieces.push(store_piece([a, b], &u, &v, work)?);
    }
    Ok(pieces)
}

struct InverseAxis {
    origin: Q,
    width: Q,
    inverse_minus_one: Q,
    scaled_inverse: Q,
}

fn inverse_axis([a, b]: [f64; 2], lambda: f64, work: &mut Work) -> exact::PResult<InverseAxis> {
    let origin = work.lift(a)?;
    let end = work.lift(b)?;
    let width = work.op(&end, &origin, |a, b| a - b)?;
    let lambda = work.lift(lambda)?;
    let one = Q::one();
    let inverse = work.op(&one, &lambda, |a, b| a / b)?;
    let inverse_minus_one = work.op(&inverse, &one, |a, b| a - b)?;
    let scaled_inverse = work.op(&width, &inverse, |a, b| a * b)?;
    Ok(InverseAxis {
        origin,
        width,
        inverse_minus_one,
        scaled_inverse,
    })
}

fn original_span(
    source: &Curve,
    span: usize,
    [a, b]: [f64; 2],
    work: &mut Work,
) -> exact::PResult<Vec<Poly>> {
    let a = work.lift(a)?;
    let b = work.lift(b)?;
    let width = work.op(&b, &a, |a, b| a - b)?;
    extraction::span(source, span, &a, &width, work)
}

fn remap_component(
    coordinate: &Poly,
    weight: &Poly,
    axis: &InverseAxis,
    degree: usize,
    work: &mut Work,
) -> std::result::Result<[Poly; 2], PcurvePullbackRefusal> {
    let shifted = exact::add(coordinate, &scale(weight, &axis.origin, work)?, true, work)?;
    let extent = scale(weight, &axis.width, work)?;
    let remaining = exact::add(&extent, &shifted, true, work)?;
    for bound in [&shifted, &remaining] {
        if poly::bernstein(bound, degree, work)
            .map_err(Refusal::from)?
            .iter()
            .any(Signed::is_negative)
        {
            return Err(PcurvePullbackRefusal::UnprovedChartDomain);
        }
    }
    // D = width W + (1/lambda - 1)(H - a W). The chart proof gives
    // D > 0 because it is a positive linear combination of the two sides.
    let denominator = exact::add(
        &extent,
        &scale(&shifted, &axis.inverse_minus_one, work)?,
        false,
        work,
    )?;
    let numerator = exact::add(
        &scale(&denominator, &axis.origin, work)?,
        &scale(&shifted, &axis.scaled_inverse, work)?,
        false,
        work,
    )?;
    Ok([numerator, denominator])
}

fn scale(poly: &Poly, factor: &Q, work: &mut Work) -> exact::PResult<Poly> {
    let mut scaled = poly
        .iter()
        .map(|q| work.op(q, factor, |a, b| a * b).map_err(Refusal::from))
        .collect::<exact::PResult<Vec<_>>>()?;
    poly::trim(&mut scaled);
    Ok(scaled)
}

fn store_piece(
    interval: [f64; 2],
    [nu, du]: &[Poly; 2],
    [nv, dv]: &[Poly; 2],
    work: &mut Work,
) -> exact::PResult<Curve> {
    let target = [
        poly::multiply(nu, dv, work)?,
        poly::multiply(nv, du, work)?,
        poly::multiply(du, dv, work)?,
    ];
    let reduced = exact::reduced(&target, work)?;
    let degree = reduced
        .iter()
        .map(|p| p.len().saturating_sub(1))
        .max()
        .unwrap_or(0)
        .max(1);
    if degree > 25 {
        return Err(Refusal::OutputDegreeLimit);
    }
    let channels = reduced
        .iter()
        .map(|p| poly::bernstein(p, degree, work).map_err(Into::into))
        .collect::<exact::PResult<Vec<_>>>()?;
    let (control_points, weights) = rational_edit::store(&channels, work)?;
    let [a, b] = interval;
    let out = Curve {
        degree,
        knots: [vec![a; degree + 1], vec![b; degree + 1]].concat(),
        control_points,
        weights,
        periodic: false,
    };
    out.validate().map_err(|_| Refusal::InvalidOutput)?;
    proof::stored(&out, &target, work)?;
    Ok(out)
}
