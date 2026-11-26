//! Pull back retained UV curves under independent Möbius chart clocks.
use super::{chart_nonnegative::nonnegative, mobius::validate_clock};
use crate::exact_geometry::{
    exact_derivative::{self as exact, Refusal, extraction, proof},
    rational_edit,
    rational_factor::polynomial::{self as poly, Poly},
};
use crate::{Result, check, core::bounded_rational::Work, curve::Curve};
use num_rational::BigRational as Q;
use num_traits::One;

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
    validate_chart(source, domain)?;
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

#[derive(Clone, Debug)]
pub struct PcurveChartCertificate {
    /// True for containment, false if a span leaves the closed rectangle.
    /// Absent on arithmetic refusal.
    pub inside: Option<bool>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Decide whether every one-sided span of a rational UV curve lies entirely
/// in a closed rectangular chart, including exact contacts at even roots.
/// The result is independent of whether a transferred curve can be stored.
pub fn certify_pcurve_chart_domain(
    source: &Curve,
    domain: [[f64; 2]; 2],
    max_work: u64,
) -> Result<PcurveChartCertificate> {
    validate_chart(source, domain)?;
    validate_clock(&[], max_work)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let proof = certify_chart(source, domain, &mut work);
    let (inside, refusal) = match proof {
        Ok(inside) => (Some(inside), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(PcurveChartCertificate {
        inside,
        work_used: work.used,
        refusal,
    })
}

pub(super) fn validate_chart(source: &Curve, domain: [[f64; 2]; 2]) -> Result<()> {
    source.validate()?;
    check(
        source.control_points[0].len() == 2,
        "A retained pcurve needs exactly two coordinates",
    )?;
    check(
        domain
            .iter()
            .all(|[a, b]| a.is_finite() && b.is_finite() && a < b),
        "A pcurve chart needs two finite nonempty UV intervals",
    )?;
    Ok(())
}

pub(super) fn active_spans(source: &Curve) -> impl Iterator<Item = (usize, [f64; 2])> + '_ {
    (source.degree..source.control_points.len()).filter_map(|span| {
        let [a, b] = [source.knots[span], source.knots[span + 1]];
        (a < b).then_some((span, [a, b]))
    })
}

pub(super) fn certify_chart(source: &Curve, domain: [[f64; 2]; 2], work: &mut Work) -> exact::PResult<bool> {
    let a = ChartInterval::new(domain[0], work)?;
    let b = ChartInterval::new(domain[1], work)?;
    for (span, interval) in active_spans(source) {
        let h = original_span(source, span, interval, work)?;
        if a.contained(&h[0], &h[2], source.degree, work)?.is_none()
            || b.contained(&h[1], &h[2], source.degree, work)?.is_none()
        {
            return Ok(false);
        }
    }
    Ok(true)
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
    for (span, [a, b]) in active_spans(source) {
        let h = original_span(source, span, [a, b], work)?;
        let u = remap_component(&h[0], &h[2], &axes[0], source.degree, work)?;
        let v = remap_component(&h[1], &h[2], &axes[1], source.degree, work)?;
        pieces.push(store_piece([a, b], &u, &v, work)?);
    }
    Ok(pieces)
}

pub(super) struct ChartInterval {
    origin: Q,
    width: Q,
}

impl ChartInterval {
    pub(super) fn new([a, b]: [f64; 2], work: &mut Work) -> exact::PResult<Self> {
        let origin = work.lift(a)?;
        let end = work.lift(b)?;
        let width = work.op(&end, &origin, |a, b| a - b)?;
        Ok(Self { origin, width })
    }

    pub(super) fn contained(
        &self,
        coordinate: &Poly,
        weight: &Poly,
        degree: usize,
        work: &mut Work,
    ) -> exact::PResult<Option<[Poly; 2]>> {
        let shifted = exact::add(coordinate, &scale(weight, &self.origin, work)?, true, work)?;
        let extent = scale(weight, &self.width, work)?;
        let remaining = exact::add(&extent, &shifted, true, work)?;
        if nonnegative(&shifted, degree, work)? && nonnegative(&remaining, degree, work)? {
            Ok(Some([shifted, extent]))
        } else {
            Ok(None)
        }
    }
}

pub(super) struct InverseAxis {
    interval: ChartInterval,
    new_origin: Q,
    inverse_minus_one: Q,
    scaled_inverse: Q,
}

fn inverse_axis(domain: [f64; 2], lambda: f64, work: &mut Work) -> exact::PResult<InverseAxis> {
    relocated_axis(domain, domain, lambda, work)
}

pub(super) fn relocated_axis(
    old: [f64; 2],
    new: [f64; 2],
    lambda: f64,
    work: &mut Work,
) -> exact::PResult<InverseAxis> {
    let interval = ChartInterval::new(old, work)?;
    let output = if old == new {
        ChartInterval {
            origin: interval.origin.clone(),
            width: interval.width.clone(),
        }
    } else {
        ChartInterval::new(new, work)?
    };
    let lambda = work.lift(lambda)?;
    let one = Q::one();
    let inverse = work.op(&one, &lambda, |a, b| a / b)?;
    let inverse_minus_one = work.op(&inverse, &one, |a, b| a - b)?;
    let scaled_inverse = work.op(&output.width, &inverse, |a, b| a * b)?;
    Ok(InverseAxis {
        interval,
        new_origin: output.origin,
        inverse_minus_one,
        scaled_inverse,
    })
}

pub(super) fn original_span(
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

pub(super) fn remap_component(
    coordinate: &Poly,
    weight: &Poly,
    axis: &InverseAxis,
    degree: usize,
    work: &mut Work,
) -> std::result::Result<[Poly; 2], PcurvePullbackRefusal> {
    let [shifted, extent] = axis
        .interval
        .contained(coordinate, weight, degree, work)?
        .ok_or(PcurvePullbackRefusal::UnprovedChartDomain)?;
    // D = width W + (1/lambda - 1)(H - a W). The chart proof gives
    // D > 0 because it is a positive linear combination of the two sides.
    let denominator = exact::add(
        &extent,
        &scale(&shifted, &axis.inverse_minus_one, work)?,
        false,
        work,
    )?;
    let numerator = exact::add(
        &scale(&denominator, &axis.new_origin, work)?,
        &scale(&shifted, &axis.scaled_inverse, work)?,
        false,
        work,
    )?;
    Ok([numerator, denominator])
}

pub(super) fn scale(poly: &Poly, factor: &Q, work: &mut Work) -> exact::PResult<Poly> {
    let mut scaled = poly
        .iter()
        .map(|q| work.op(q, factor, |a, b| a * b).map_err(Refusal::from))
        .collect::<exact::PResult<Vec<_>>>()?;
    poly::trim(&mut scaled);
    Ok(scaled)
}

pub(super) fn store_piece(
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
    store_target(interval, &target, work)
}

pub(super) fn store_target(
    interval: [f64; 2],
    target: &[Poly],
    work: &mut Work,
) -> exact::PResult<Curve> {
    let reduced = exact::reduced(target, work)?;
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
    proof::stored(&out, target, work)?;
    Ok(out)
}
