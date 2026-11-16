//! Bounded exact isolation of individual UV seam contacts.
use super::{
    UvAxis,
    chart_nonnegative::{SturmChain, interior_contact_factors},
    pcurve_mobius::active_spans,
    pcurve_seam::{seam_power, validate_seam},
};
use crate::{
    Result, check,
    core::bounded_rational::Work,
    curve::Curve,
    editing::{
        exact_derivative::{PResult, Refusal},
        rational_factor::polynomial::Poly,
    },
};
use num_rational::BigRational as Q;
use num_traits::{One, Zero};

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
    let max_width = max_width.map(|width| work.lift(width)).transpose()?;
    active_spans(source)
        .map(|(span, parameter)| {
            let power = seam_power(source, span, parameter, coordinate, &seam, work)?;
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
