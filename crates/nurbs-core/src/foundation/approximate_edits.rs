//! Native approximate edit decisions and rollback results.
use super::periodic_edits::SeamCertificate;
use super::{
    Axis, Curve, Result, Surface, ToleranceContext, check, context,
    exact_curve, exact_surface, numeric, rebuild_candidate, rebuild_surface_error,
    refit_curve, seam_certificate,
};
#[cfg(feature = "codec")]
pub mod serialization;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SurfaceSimplification {
    Remove,
    Reduce,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EditOperation {
    KnotRemoval,
    CurveRebuild,
    DegreeReduction,
    SurfaceKnotRemoval,
    SurfaceDegreeReduction,
    SurfaceRebuild,
}
pub struct CurveRebuildEvidence {
    pub wrapped_storage: bool,
    pub seam: Option<SeamCertificate>,
}
pub struct EditCertificate {
    pub operation: EditOperation,
    pub accepted: bool,
    pub exact_zero_recognized: bool,
    pub error_upper: f64,
    pub budget: f64,
    pub tolerance: ToleranceContext,
    pub rebuild: Option<CurveRebuildEvidence>,
}
pub struct CurveEdit {
    pub curve: Curve,
    pub certificate: EditCertificate,
}
pub struct SurfaceEdit {
    pub surface: Surface,
    pub certificate: EditCertificate,
}

pub fn remove_curve_knot_report(
    source: &Curve,
    knot: f64,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<CurveEdit> {
    source.validate()?;
    check(
        !source.periodic,
        "Certified removal currently requires non-periodic storage",
    )?;
    check(
        max_error.is_finite() && max_error >= 0.,
        "Maximum error must be finite and nonnegative",
    )?;
    let domain = source.domain();
    check(
        knot > domain[0] && knot < domain[1],
        "Only interior knots can be removed",
    )?;
    let Some(index) = source.knots.iter().position(|value| *value == knot) else {
        return Err(crate::input("Knot is absent"));
    };
    let mut knots = source.knots.clone();
    knots.remove(index);
    let candidate = refit_curve(source, source.degree, knots)?;
    // Certify the actual returned candidate, not a rounded reinsertion of it.
    let report = crate::curve_deviation::inspect(
        source,
        &candidate,
        (max_error * 0.01).max(1e-12),
        4096,
    )?;
    let exact = report.bounds == [0., 0.];
    let error = report.bounds[1];
    let accepted = error <= max_error;
    let tolerance = context(tolerance);
    Ok(CurveEdit {
        curve: if accepted { candidate } else { source.clone() },
        certificate: EditCertificate {
            operation: EditOperation::KnotRemoval,
            accepted,
            exact_zero_recognized: exact,
            error_upper: error,
            budget: max_error,
            tolerance,
            rebuild: None,
        },
    })
}

/// Refit into a uniform basis with acceptance checked over the full parameter domain.
pub fn rebuild_curve_report(
    source: &Curve,
    degree: usize,
    control_count: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<CurveEdit> {
    source.validate()?;
    check(
        max_error.is_finite() && max_error >= 0.,
        "Maximum error must be finite and nonnegative",
    )?;
    let candidate = rebuild_candidate(source, degree, control_count)?;
    let report = crate::curve_deviation::inspect(
        source,
        &candidate,
        (max_error * 0.01).max(1e-12),
        4096,
    )?;
    let exact = report.bounds == [0., 0.];
    let error = report.bounds[1];
    numeric(error.is_finite(), "Rebuild deviation bound overflowed")?;
    let accepted = error <= max_error;
    let tolerance = context(tolerance);
    let rebuild = CurveRebuildEvidence {
        wrapped_storage: source.periodic,
        seam: if source.periodic {
            Some(seam_certificate(&candidate)?)
        } else {
            None
        },
    };
    Ok(CurveEdit {
        curve: if accepted { candidate } else { source.clone() },
        certificate: EditCertificate {
            operation: EditOperation::CurveRebuild,
            accepted,
            exact_zero_recognized: exact,
            error_upper: error,
            budget: max_error,
            tolerance,
            rebuild: Some(rebuild),
        },
    })
}

pub fn reduce_curve_degree_report(
    source: &Curve,
    degree: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<CurveEdit> {
    source.validate()?;
    check(
        !source.periodic,
        "Certified reduction currently requires non-periodic storage",
    )?;
    check(
        degree >= 1 && degree < source.degree,
        "Target degree must be in [1, source degree)",
    )?;
    check(
        max_error.is_finite() && max_error >= 0.,
        "Maximum error must be finite and nonnegative",
    )?;
    let remove = source.degree - degree;
    let mut knots = source.knots.clone();
    for _ in 0..remove {
        let mut index = knots.len();
        let mut previous = None;
        while index > 0 {
            index -= 1;
            if previous != Some(knots[index]) {
                previous = Some(knots[index]);
                knots.remove(index);
            }
        }
    }
    let candidate = refit_curve(source, degree, knots)?;
    // Recognize exact reductions exactly, as the archived contract requires:
    // elevate the candidate back and compare control points, and only then
    // fall back to the conservative interval bound for inexact candidates.
    let reconstructed = candidate.elevate(source.degree)?;
    let (exact, error) = if exact_curve(source, &reconstructed) {
        (true, 0.)
    } else {
        // Bound the returned candidate directly; rounded elevation is not its image.
        let report = crate::curve_deviation::inspect(
            source,
            &candidate,
            (max_error * 0.01).max(1e-12),
            4096,
        )?;
        (report.bounds == [0., 0.], report.bounds[1])
    };
    let accepted = error <= max_error;
    let tolerance = context(tolerance);
    Ok(CurveEdit {
        curve: if accepted { candidate } else { source.clone() },
        certificate: EditCertificate {
            operation: EditOperation::DegreeReduction,
            accepted,
            exact_zero_recognized: exact,
            error_upper: error,
            budget: max_error,
            tolerance,
            rebuild: None,
        },
    })
}

pub fn reduce_surface_axis_report(
    source: &Surface,
    axis: Axis,
    operation: SurfaceSimplification,
    parameter: f64,
    degree: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<SurfaceEdit> {
    source.validate()?;
    check(
        max_error.is_finite() && max_error >= 0.,
        "Maximum error must be finite and nonnegative",
    )?;
    let (source_degree, periodic) = match axis {
        Axis::U => (source.degree_u, source.periodic_u),
        Axis::V => (source.degree_v, source.periodic_v),
    };
    check(
        !periodic,
        "Use periodic surface editing for a periodic axis",
    )?;
    if operation == SurfaceSimplification::Reduce {
        check(
            degree >= 1 && degree < source_degree,
            "Target degree must be in [1, source degree)",
        )?;
    } else {
        check(parameter.is_finite(), "Knot parameter must be finite")?;
        let (knots, count) = match axis {
            Axis::U => (&source.knots_u, source.control_points.len()),
            Axis::V => (&source.knots_v, source.control_points[0].len()),
        };
        check(parameter > knots[source_degree] && parameter < knots[count],
            "Only interior surface knots can be removed")?;
    }
    let candidate = source.edit_axis(axis, |curve| {
        if operation == SurfaceSimplification::Remove {
            let mut knots = curve.knots.clone();
            let index = knots
                .iter()
                .position(|value| *value == parameter)
                .ok_or_else(|| crate::input("Knot is absent"))?;
            knots.remove(index);
            refit_curve(curve, curve.degree, knots)
        } else {
            let mut knots = curve.knots.clone();
            for _ in 0..curve.degree - degree {
                let mut index = knots.len();
                let mut previous = None;
                while index > 0 {
                    index -= 1;
                    if previous != Some(knots[index]) {
                        previous = Some(knots[index]);
                        knots.remove(index);
                    }
                }
            }
            refit_curve(curve, degree, knots)
        }
    })?;
    // Recognize exact edits exactly, as the archived contract requires:
    // reconstruct the source degrees/knots and compare control points, and
    // only then fall back to the conservative positional bound.
    let reconstructed = candidate.edit_axis(axis, |curve| {
        if operation == SurfaceSimplification::Remove {
            curve.insert(parameter, 1)
        } else {
            curve.elevate(source_degree)
        }
    })?;
    let exact = exact_surface(source, &reconstructed);
    // Bitwise equality of the reconstructed net is reported as
    // `exact_zero_recognized` evidence, but a rounded reconstruction
    // coincidence is not a proven zero bound (the archived contract in
    // docs/design/nurbs-knot-removal.md), so the certificate always carries
    // the outward-rounded positional bound: a zero budget is never accepted.
    let error = crate::continuity::deviation::positional_upper(source, &candidate)?;
    let accepted = error <= max_error;
    let tolerance = context(tolerance);
    Ok(SurfaceEdit {
        surface: if accepted { candidate } else { source.clone() },
        certificate: EditCertificate {
            operation: if operation == SurfaceSimplification::Remove {
                EditOperation::SurfaceKnotRemoval
            } else {
                EditOperation::SurfaceDegreeReduction
            },
            accepted,
            exact_zero_recognized: exact,
            error_upper: error,
            budget: max_error,
            tolerance,
            rebuild: None,
        },
    })
}

pub fn rebuild_surface_report(
    source: &Surface,
    axis: Axis,
    degree: usize,
    control_count: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<SurfaceEdit> {
    source.validate()?;
    check(
        max_error.is_finite() && max_error >= 0.,
        "Maximum error must be finite and nonnegative",
    )?;
    let candidate = source.edit_axis(axis, |curve| {
        rebuild_candidate(curve, degree, control_count)
    })?;
    let exact = exact_surface(source, &candidate);
    let error = if exact {
        0.
    } else {
        rebuild_surface_error(source, &candidate)?
    };
    let accepted = error <= max_error;
    let tolerance = context(tolerance);
    Ok(SurfaceEdit {
        surface: if accepted { candidate } else { source.clone() },
        certificate: EditCertificate {
            operation: EditOperation::SurfaceRebuild,
            accepted,
            exact_zero_recognized: exact,
            error_upper: error,
            budget: max_error,
            tolerance,
            rebuild: None,
        },
    })
}

/// Remove one existing interior knot along a non-periodic surface axis.
pub fn remove_surface_knot_report(
    source: &Surface, axis: Axis, knot: f64, max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<SurfaceEdit> {
    reduce_surface_axis_report(source, axis, SurfaceSimplification::Remove,
        knot, 0, max_error, tolerance)
}

/// Refit a non-periodic surface axis to a smaller degree with a continuous bound.
pub fn reduce_surface_degree_report(
    source: &Surface, axis: Axis, degree: usize, max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<SurfaceEdit> {
    reduce_surface_axis_report(source, axis, SurfaceSimplification::Reduce,
        0., degree, max_error, tolerance)
}
