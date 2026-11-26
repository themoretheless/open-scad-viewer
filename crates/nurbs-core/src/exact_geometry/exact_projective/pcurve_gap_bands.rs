//! Certified parameter bands where a paired seam exceeds its gap tolerance.
use super::{
    PcurvePullbackRefusal, chart_nonnegative::sum, pcurve_agreement::compare_surface_fields,
    pcurve_gap::gap_residual, pcurve_seam_isolation::isolate_power,
};
use crate::{
    Result, check,
    core::bounded_rational::Work,
    curve::Curve,
    exact_geometry::{
        exact_derivative::{PResult, Refusal},
        rational_factor::polynomial::Poly,
    },
    surface::Surface,
};
use num_rational::BigRational as Q;
use num_traits::{Signed, Zero};
use std::ops::ControlFlow;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PcurveGapBoundary {
    /// Enclosure of the true boundary parameter. Equal endpoints are exact.
    pub bracket: [f64; 2],
    /// Whether the true boundary point itself violates the tolerance.
    pub included: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PcurveGapBand {
    pub start: PcurveGapBoundary,
    pub end: PcurveGapBoundary,
}

#[derive(Clone, Debug)]
pub struct PcurveGapBandsReport {
    /// Ordered strict violation bands, split at original common partition
    /// boundaries. Absent if the entire isolation could not be proved.
    pub bands: Option<Vec<PcurveGapBand>>,
    pub work_used: u64,
    pub refusal: Option<PcurvePullbackRefusal>,
}

/// Locate every connected strict gap violation within each common parameter
/// span. Nonbinary threshold contacts get single-root parameter brackets of
/// the requested width. Equality at a tangency is excluded even when the
/// gap exceeds the tolerance on both sides.
pub fn isolate_pcurve_seam_gap(
    left: (&Surface, &Curve),
    right: (&Surface, &Curve),
    tolerance: f64,
    max_width: f64,
    max_work: u64,
) -> Result<PcurveGapBandsReport> {
    check(
        tolerance.is_finite() && tolerance >= 0.,
        "Seam gap tolerance must be finite and nonnegative",
    )?;
    check(
        max_width.is_finite() && max_width > 0.,
        "Seam gap isolation needs a positive finite parameter width",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let mut bands = Vec::new();
    let result = compare_surface_fields(left, right, false, &mut work, |parameter, a, b, work| {
        let limit = work.lift(tolerance).map_err(Refusal::from)?;
        let square = work
            .op(&limit, &limit, |a, b| a * b)
            .map_err(Refusal::from)?;
        let width = work.lift(max_width).map_err(Refusal::from)?;
        let residual = gap_residual(a, b, &square, work)?;
        append_bands(&residual, parameter, &width, &mut bands, work)?;
        Ok(ControlFlow::<()>::Continue(()))
    })?;
    let (bands, refusal) = match result {
        Ok(_) => (Some(bands), None),
        Err(reason) => (None, Some(reason)),
    };
    Ok(PcurveGapBandsReport {
        bands,
        work_used: work.used,
        refusal,
    })
}

pub(super) fn append_bands(
    residual: &Poly,
    parameter: [f64; 2],
    width: &Q,
    out: &mut Vec<PcurveGapBand>,
    work: &mut Work,
) -> PResult<()> {
    let Some(initial) = residual.iter().find(|q| !q.is_zero()) else {
        return Ok(());
    };
    let mut violates = initial.is_negative();
    let mut start = PcurveGapBoundary {
        bracket: [parameter[0]; 2],
        included: residual[0].is_negative(),
    };
    for root in isolate_power(residual, parameter, Some(width), work)? {
        let boundary = PcurveGapBoundary {
            bracket: root.parameter,
            included: false,
        };
        if violates {
            out.push(PcurveGapBand {
                start,
                end: boundary,
            });
        }
        violates ^= root.crosses;
        start = boundary;
    }
    if violates {
        out.push(PcurveGapBand {
            start,
            end: PcurveGapBoundary {
                bracket: [parameter[1]; 2],
                included: sum(residual, work)?.is_negative(),
            },
        });
    }
    Ok(())
}
