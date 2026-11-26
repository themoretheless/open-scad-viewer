//! Bounded exact isolation of individual UV seam contacts.
use super::{
    UvAxis,
    chart_nonnegative::{SturmChain, interior_contact_factors},
    mobius::validate_clock,
    pcurve_mobius::{active_spans, original_span, scale},
    pcurve_seam::{seam_power, validate_seam},
};
use crate::{
    Result, check,
    core::bounded_rational::Work,
    curve::Curve,
    exact_geometry::{
        exact_derivative::{PResult, Refusal, add},
        rational_factor::polynomial::{Poly, multiply},
    },
};
use num_rational::BigRational as Q;
use num_traits::{One, Signed, Zero};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PcurveSeamRoot {
    /// One root in (lo, hi], or an exact root when lo equals hi.
    pub parameter: [f64; 2],
    pub crosses: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum PcurveSeamIsolation {
    Coincident,
    Discrete {
        at_start: bool,
        at_end: bool,
        roots: Vec<PcurveSeamRoot>,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PcurveSeamIsolatedSpan {
    pub parameter: [f64; 2],
    pub contact: PcurveSeamIsolation,
}

#[derive(Clone, Debug)]
pub struct PcurveSeamIsolationReport {
    pub spans: Option<Vec<PcurveSeamIsolatedSpan>>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Enclose each interior root of the exact homogeneous seam equation
/// in the original curve's absolute parameter, with exact multiplicity parity.
/// Endpoint contacts belong to their one-sided original span. Coincident
/// spans are retained as intervals. Adjacent binary64 values may preclude
/// isolation at the requested width, in which case the report refuses.
pub fn isolate_pcurve_seam(
    source: &Curve,
    axis: UvAxis,
    seam: f64,
    max_width: f64,
    max_work: u64,
) -> Result<PcurveSeamIsolationReport> {
    let coordinate = validate_seam(source, axis, seam, max_work)?;
    check(
        max_width.is_finite() && max_width > 0.,
        "Seam isolation needs a positive finite parameter width",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let (spans, refusal) = match isolate(source, coordinate, seam, Some(max_width), &mut work) {
        Ok(spans) => (Some(spans), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(PcurveSeamIsolationReport {
        spans,
        refusal,
        work_used: work.used,
    })
}

pub(super) fn isolate(
    source: &Curve,
    coordinate: usize,
    seam: f64,
    max_width: Option<f64>,
    work: &mut Work,
) -> PResult<Vec<PcurveSeamIsolatedSpan>> {
    let seam = work.lift(seam)?;
    isolate_equation(source, max_width, work, |span, parameter, work| {
        seam_power(source, span, parameter, coordinate, &seam, work)
    })
}

/// Isolate contacts of a rational UV curve with a line a*u + b*v + c = 0.
/// All finite authored coefficients are used exactly. Each span retains
/// endpoint ownership, and coincident spans are reported as full intervals.
pub fn isolate_pcurve_line(
    source: &Curve,
    coefficients: [f64; 3],
    max_width: f64,
    max_work: u64,
) -> Result<PcurveSeamIsolationReport> {
    source.validate()?;
    check(
        source.control_points[0].len() == 2
            && coefficients.iter().all(|x| x.is_finite())
            && (coefficients[0] != 0. || coefficients[1] != 0.)
            && max_width.is_finite()
            && max_width > 0.,
        "Line contact needs a 2D curve, finite nonzero line normal and positive parameter width",
    )?;
    validate_clock(&[], max_work)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let result = (|| {
        let coefficients = [
            work.lift(coefficients[0])?,
            work.lift(coefficients[1])?,
            work.lift(coefficients[2])?,
        ];
        isolate_rational_pcurve_line(source, &coefficients, max_width, &mut work)
    })();
    let (spans, refusal) = match result {
        Ok(spans) => (Some(spans), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(PcurveSeamIsolationReport {
        spans,
        refusal,
        work_used: work.used,
    })
}

pub(crate) fn isolate_rational_pcurve_line(
    source: &Curve,
    coefficients: &[Q; 3],
    max_width: f64,
    work: &mut Work,
) -> PResult<Vec<PcurveSeamIsolatedSpan>> {
    isolate_equation(source, Some(max_width), work, |span, parameter, work| {
        let homogeneous = original_span(source, span, parameter, work)?;
        line_equation(&homogeneous, coefficients, work)
    })
}

fn line_equation(homogeneous: &[Poly], coefficients: &[Q; 3], work: &mut Work) -> PResult<Poly> {
    homogeneous
        .iter()
        .zip(coefficients)
        .try_fold(Poly::new(), |sum, (component, factor)| {
            let term = scale(component, factor, work)?;
            add(&sum, &term, false, work)
        })
}

#[derive(Clone, Copy)]
pub(crate) enum SpanLineCoverage {
    Endpoints,
    Contacts,
    Refined { stationary_width: f64 },
}

/// Certified coordinate ranges and exact isolated contacts of the line.
pub(crate) fn exact_span_line_ranges(
    source: &Curve,
    coefficients: &[Q; 3],
    axis: usize,
    coverage: SpanLineCoverage,
    work: &mut Work,
) -> PResult<Vec<[Q; 2]>> {
    use super::chart_nonnegative::{evaluate, sum};
    let mut result = Vec::new();
    let zero = Q::zero();
    let one = Q::one();
    for (span, parameter) in active_spans(source) {
        let homogeneous = original_span(source, span, parameter, work)?;
        let equation = line_equation(&homogeneous, coefficients, work)?;
        let coordinate = |t: &Q, work: &mut Work| -> PResult<Q> {
            let value = evaluate(&homogeneous[axis], t, work)?;
            let weight = evaluate(&homogeneous[2], t, work)?;
            if weight.is_zero() {
                return Err(Refusal::UnprovedDenominator);
            }
            Ok(work.op(&value, &weight, |a, b| a / b)?)
        };
        if equation.is_empty() {
            let start = coordinate(&zero, work)?;
            let end = coordinate(&one, work)?;
            if let SpanLineCoverage::Refined { stationary_width } = coverage {
                let (mut lo, mut hi) = if start <= end {
                    (start, end)
                } else {
                    (end, start)
                };
                for station in stationary_witnesses(
                    &homogeneous[axis],
                    &homogeneous[2],
                    stationary_width,
                    work,
                )? {
                    let value = coordinate(&station, work)?;
                    if value < lo {
                        lo = value;
                    } else if value > hi {
                        hi = value;
                    }
                }
                result.push([lo, hi]);
            } else {
                result.push([start, end]);
            }
        } else if !matches!(coverage, SpanLineCoverage::Endpoints) {
            let mut roots = rational_interior_roots(&equation, work)?;
            if equation[0].is_zero() {
                roots.push(zero.clone());
            }
            if sum(&equation, work)?.is_zero() {
                roots.push(one.clone());
            }
            for root in roots {
                let value = coordinate(&root, work)?;
                result.push([value.clone(), value]);
            }
        }
    }
    Ok(result)
}

fn stationary_witnesses(
    component: &Poly,
    weight: &Poly,
    max_width: f64,
    work: &mut Work,
) -> PResult<Vec<Q>> {
    stationary_brackets(component, weight, max_width, work)?
        .iter()
        .map(|bracket| bracket_midpoint(bracket, work))
        .collect()
}

fn stationary_brackets(
    component: &Poly,
    weight: &Poly,
    max_width: f64,
    work: &mut Work,
) -> PResult<Vec<[Q; 2]>> {
    use super::chart_nonnegative::derivative;
    let dw = derivative(weight, work)?;
    let dc = derivative(component, work)?;
    let forward = multiply(&dc, weight, work)?;
    let backward = multiply(component, &dw, work)?;
    let power = add(&forward, &backward, true, work)?;
    interior_brackets(&power, max_width, work)
}

fn interior_brackets(power: &Poly, max_width: f64, work: &mut Work) -> PResult<Vec<[Q; 2]>> {
    let exact = rational_interior_roots(power, work)?;
    if power.len() <= 2 || !exact.is_empty() {
        return Ok(exact.into_iter().map(|root| [root.clone(), root]).collect());
    }
    let width = work.lift(max_width)?;
    isolate_power(power, [0., 1.], Some(&width), work)?
        .into_iter()
        .map(|root| {
            let [lo, hi] = root.parameter;
            Ok([work.lift(lo)?, work.lift(hi)?])
        })
        .collect()
}

fn bracket_midpoint([lo, hi]: &[Q; 2], work: &mut Work) -> PResult<Q> {
    if lo == hi {
        return Ok(lo.clone());
    }
    let two = Q::from_integer(2.into());
    Ok(work.op(lo, hi, |a, b| (a + b) / &two)?)
}

mod range_bounds;
pub(crate) use range_bounds::{SpanLineBounds, span_line_bounds};

fn rational_interior_roots(power: &Poly, work: &mut Work) -> PResult<Vec<Q>> {
    let roots = match power.as_slice() {
        [c, b] => vec![work.op(c, b, |c, b| -c / b)?],
        [c, b, a] => {
            let ac = work.op(a, c, |a, c| a * c)?;
            let b2 = work.op(b, b, |a, b| a * b)?;
            let four = Q::from_integer(4.into());
            let four_ac = work.op(&ac, &four, |x, y| x * y)?;
            let d = work.op(&b2, &four_ac, |x, y| x - y)?;
            if d.is_negative() {
                return Ok(Vec::new());
            }
            work.charge(1 + (d.numer().bits() + d.denom().bits()) / 64)?;
            let sqrt = Q::new(d.numer().sqrt(), d.denom().sqrt());
            if work.op(&sqrt, &sqrt, |x, y| x * y)? != d {
                return Ok(Vec::new());
            }
            let two = Q::from_integer(2.into());
            let denominator = work.op(a, &two, |x, y| x * y)?;
            let mut roots = Vec::with_capacity(2);
            for offset in [&sqrt, &(-&sqrt)] {
                let numerator = work.op(b, offset, |b, offset| offset - b)?;
                roots.push(work.op(&numerator, &denominator, |x, y| x / y)?);
            }
            roots.dedup();
            roots
        }
        _ => Vec::new(),
    };
    Ok(roots
        .into_iter()
        .filter(|r| r.is_positive() && r < &Q::one())
        .collect())
}

fn isolate_equation(
    source: &Curve,
    max_width: Option<f64>,
    work: &mut Work,
    mut power_on_span: impl FnMut(usize, [f64; 2], &mut Work) -> PResult<Poly>,
) -> PResult<Vec<PcurveSeamIsolatedSpan>> {
    let max_width = max_width.map(|width| work.lift(width)).transpose()?;
    active_spans(source)
        .map(|(span, parameter)| {
            let power = power_on_span(span, parameter, work)?;
            let contact = if power.is_empty() {
                PcurveSeamIsolation::Coincident
            } else {
                let at_start = power[0].is_zero();
                let at_end = super::chart_nonnegative::sum(&power, work)?.is_zero();
                let roots = isolate_power(&power, parameter, max_width.as_ref(), work)?;
                PcurveSeamIsolation::Discrete {
                    at_start,
                    at_end,
                    roots,
                }
            };
            Ok(PcurveSeamIsolatedSpan { parameter, contact })
        })
        .collect()
}

pub(super) fn isolate_power(
    power: &Poly,
    domain: [f64; 2],
    max_width: Option<&Q>,
    work: &mut Work,
) -> PResult<Vec<PcurveSeamRoot>> {
    let [distinct, odd] = interior_contact_factors(power, work)?;
    Isolator {
        distinct: SturmChain::new(distinct, work)?,
        odd: SturmChain::new(odd, work)?,
        domain,
        origin: work.lift(domain[0])?,
        width: distance(domain, work)?,
        max_width,
    }
    .roots(work)
}

struct Isolator<'a> {
    distinct: SturmChain,
    odd: SturmChain,
    domain: [f64; 2],
    origin: Q,
    width: Q,
    max_width: Option<&'a Q>,
}

#[derive(Clone, Copy)]
struct Endpoint {
    t: f64,
    variations: [usize; 2],
    is_root: bool,
}

impl Isolator<'_> {
    fn endpoint(&self, t: f64, work: &mut Work) -> PResult<Endpoint> {
        let local = if t == self.domain[0] {
            Q::zero()
        } else if t == self.domain[1] {
            Q::one()
        } else {
            let t = work.lift(t)?;
            let shifted = work.op(&t, &self.origin, |a, b| a - b)?;
            work.op(&shifted, &self.width, |a, b| a / b)?
        };
        let (all, is_root) = self.distinct.at(&local, work)?;
        let (odd, _) = self.odd.at(&local, work)?;
        Ok(Endpoint {
            t,
            variations: [all, odd],
            is_root,
        })
    }

    fn within_width(&self, parameter: [f64; 2], work: &mut Work) -> PResult<bool> {
        match self.max_width {
            Some(width) => Ok(distance(parameter, work)? <= *width),
            None => Ok(false),
        }
    }

    fn roots(&self, work: &mut Work) -> PResult<Vec<PcurveSeamRoot>> {
        let mut pending = vec![[
            self.endpoint(self.domain[0], work)?,
            self.endpoint(self.domain[1], work)?,
        ]];
        let mut roots = Vec::new();
        while let Some([left, right]) = pending.pop() {
            let [all, odd] = [0, 1].map(|i| left.variations[i].checked_sub(right.variations[i]));
            let (Some(all), Some(odd)) = (all, odd) else {
                return Err(Refusal::UnprovedIdentity);
            };
            if odd > all {
                return Err(Refusal::UnprovedIdentity);
            }
            if all == 0 {
                continue;
            }
            let parameter = [left.t, right.t];
            if all == 1 && (right.is_root || self.within_width(parameter, work)?) {
                roots.push(PcurveSeamRoot {
                    parameter: if right.is_root {
                        [right.t; 2]
                    } else {
                        parameter
                    },
                    crosses: odd == 1,
                });
                continue;
            }
            let mid = left.t.midpoint(right.t);
            if mid <= left.t || mid >= right.t {
                return Err(Refusal::NotRepresentable);
            }
            let center = self.endpoint(mid, work)?;
            pending.push([center, right]);
            pending.push([left, center]);
        }
        Ok(roots)
    }
}

fn distance([a, b]: [f64; 2], work: &mut Work) -> PResult<Q> {
    let a = work.lift(a)?;
    let b = work.lift(b)?;
    Ok(work.op(&b, &a, |a, b| a - b)?)
}

mod quadratic_contacts;
pub use quadratic_contacts::AlgebraicBoundaryClock;
pub(crate) use quadratic_contacts::quadratic_span_contacts;
