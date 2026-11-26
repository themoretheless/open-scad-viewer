//! Exact contact multiplicity classification for rational UV curves.
use super::{
    chart_nonnegative::{contact_counts, sum},
    mobius::validate_clock,
    pcurve_mobius::{active_spans, original_span, scale},
};
use crate::{
    Result, check,
    core::bounded_rational::Work,
    curve::Curve,
    exact_geometry::{
        exact_derivative::{self as exact, Refusal},
        rational_factor::polynomial::Poly,
    },
};
use num_rational::BigRational as Q;
use num_traits::Zero;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UvAxis {
    U,
    V,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PcurveSeamContact {
    Coincident,
    Discrete {
        /// Distinct interior roots of odd multiplicity.
        crossings: usize,
        /// Distinct interior roots of even multiplicity.
        touches: usize,
        at_start: bool,
        at_end: bool,
    },
}

#[derive(Clone, Debug, PartialEq)]
pub struct PcurveSeamSpan {
    pub parameter: [f64; 2],
    pub contact: PcurveSeamContact,
}

#[derive(Clone, Debug)]
pub struct PcurveSeamReport {
    /// Original nonempty spans, including spans with no contact. At a shared
    /// knot the neighboring traces each own their corresponding endpoint.
    pub spans: Option<Vec<PcurveSeamSpan>>,
    pub work_used: u64,
    pub refusal: Option<Refusal>,
}

/// Determine exact contacts with U = seam or V = seam. Counts use the
/// homogeneous numerator for the original curve, whose weight is positive.
/// The whole result is withheld on arithmetic exhaustion.
pub fn classify_pcurve_seam(
    source: &Curve,
    axis: UvAxis,
    seam: f64,
    max_work: u64,
) -> Result<PcurveSeamReport> {
    let coordinate = validate_seam(source, axis, seam, max_work)?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let (spans, refusal) = match classify(source, coordinate, seam, &mut work) {
        Ok(spans) => (Some(spans), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(PcurveSeamReport {
        spans,
        work_used: work.used,
        refusal,
    })
}

pub(super) fn validate_seam(
    source: &Curve,
    axis: UvAxis,
    seam: f64,
    max_work: u64,
) -> Result<usize> {
    source.validate()?;
    check(
        source.control_points[0].len() == 2,
        "A UV seam needs a two-dimensional curve",
    )?;
    check(seam.is_finite(), "A UV seam coordinate must be finite")?;
    validate_clock(&[], max_work)?;
    Ok(match axis {
        UvAxis::U => 0,
        UvAxis::V => 1,
    })
}

pub(super) fn seam_power(
    source: &Curve,
    span: usize,
    parameter: [f64; 2],
    coordinate: usize,
    seam: &Q,
    work: &mut Work,
) -> exact::PResult<Poly> {
    let h = original_span(source, span, parameter, work)?;
    let offset = scale(&h[2], seam, work)?;
    exact::add(&h[coordinate], &offset, true, work)
}

fn classify(
    source: &Curve,
    coordinate: usize,
    seam: f64,
    work: &mut Work,
) -> exact::PResult<Vec<PcurveSeamSpan>> {
    let seam = work.lift(seam)?;
    active_spans(source)
        .map(|(span, parameter)| {
            let power = seam_power(source, span, parameter, coordinate, &seam, work)?;
            let contact = if power.is_empty() {
                PcurveSeamContact::Coincident
            } else {
                let [distinct, crossings] = contact_counts(&power, work)?;
                PcurveSeamContact::Discrete {
                    crossings,
                    touches: distinct
                        .checked_sub(crossings)
                        .ok_or(Refusal::UnprovedIdentity)?,
                    at_start: power[0].is_zero(),
                    at_end: sum(&power, work)?.is_zero(),
                }
            };
            Ok(PcurveSeamSpan { parameter, contact })
        })
        .collect()
}
