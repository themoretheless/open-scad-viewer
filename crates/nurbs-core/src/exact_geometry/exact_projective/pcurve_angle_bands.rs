//! Certified angular violation bands along paired rational UV traces.
use super::{
    PcurveGapBand, PcurvePullbackRefusal, pcurve_agreement::compare_surface_fields,
    pcurve_angle::angle_residual, pcurve_gap_bands::append_bands, pcurve_tangent::singular,
};
use crate::{
    Result, check, core::bounded_rational::Work, curve::Curve, exact_geometry::exact_derivative::Refusal,
    surface::Surface,
};
use std::ops::ControlFlow;

#[derive(Clone, Debug, PartialEq)]
pub enum PcurveAngleBandsOutcome {
    /// Ordered strict violations, retaining the common partition boundaries.
    Bands(Vec<PcurveGapBand>),
    Singular {
        parameter: [f64; 2],
        face: usize,
    },
    Refused(PcurvePullbackRefusal),
}

#[derive(Clone, Debug, PartialEq)]
pub struct PcurveAngleBandsReport {
    pub outcome: PcurveAngleBandsOutcome,
    pub work_used: u64,
}

/// Locate every strict exceedance of the permitted acute tangent-plane
/// angle. Supply the square of the permitted angle's cosine. Each
/// nonrepresentable threshold root is enclosed in a single-root bracket
/// of at most `max_width` in the original curve parameter. A singular
/// normal or arithmetic refusal withholds all bands.
pub fn isolate_pcurve_seam_angle(
    left: (&Surface, &Curve),
    right: (&Surface, &Curve),
    min_cosine_squared: f64,
    max_width: f64,
    max_work: u64,
) -> Result<PcurveAngleBandsReport> {
    check(
        (0.0..=1.0).contains(&min_cosine_squared),
        "Seam angle requires a squared cosine in [0, 1]",
    )?;
    check(
        max_width.is_finite() && max_width > 0.,
        "Seam angle isolation needs a positive finite parameter width",
    )?;
    let mut work = Work {
        used: 0,
        limit: max_work,
    };
    let mut bands = Vec::new();
    let result = compare_surface_fields(left, right, true, &mut work, |parameter, a, b, work| {
        let [a, b] = [&a[4..], &b[4..]];
        for (face, normal) in [a, b].into_iter().enumerate() {
            if singular(normal, work)? {
                return Ok(ControlFlow::Break(PcurveAngleBandsOutcome::Singular {
                    parameter,
                    face,
                }));
            }
        }
        let limit = work.lift(min_cosine_squared).map_err(Refusal::from)?;
        let width = work.lift(max_width).map_err(Refusal::from)?;
        let residual = angle_residual(a, b, &limit, work)?;
        append_bands(&residual, parameter, &width, &mut bands, work)?;
        Ok(ControlFlow::Continue(()))
    })?;
    let outcome = match result {
        Ok(ControlFlow::Continue(())) => PcurveAngleBandsOutcome::Bands(bands),
        Ok(ControlFlow::Break(reason)) => reason,
        Err(reason) => PcurveAngleBandsOutcome::Refused(reason),
    };
    Ok(PcurveAngleBandsReport {
        outcome,
        work_used: work.used,
    })
}
