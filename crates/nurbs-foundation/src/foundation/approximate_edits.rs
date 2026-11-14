//! Native approximate edit decisions and rollback results.
use super::periodic_edits::SeamCertificate;
use super::{
    Axis, Curve, Result, Surface, ToleranceContext, check, context,
    exact_surface, numeric, rebuild_candidate, rebuild_surface_error,
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
    let report = crate::distance::curve_deviation::inspect(
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
    let report = crate::distance::curve_deviation::inspect(
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
    // Rounded re-elevation is only a proposal filter, never zero-error proof.
    // Compare the actual original and candidate definitions independently.
    let reconstructed = candidate.elevate(source.degree)?;
    let exact = if super::exact_curve(source, &reconstructed) {
        crate::core::curve_identity::compare_definitions(
            source, &candidate, 0., cad_predicates::MAX_WORK, false,
        )?.outcome == cad_predicates::BezierIdentity::Equal
    } else { false };
    let (exact, error) = if exact {
        (true, 0.)
    } else {
        // Bound the returned candidate directly; rounded elevation is not its image.
        let report = crate::distance::curve_deviation::inspect(
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
    let error = crate::foundation::surface_deviation::positional_upper(source, &candidate)?;
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

/// Remove a set of axis knots in one refit. The returned candidate is admitted
/// only by a fresh whole-surface bound against the ORIGINAL coefficients.
/// Target knots must be a multiplicity-subset with unchanged clamped ends.
pub fn remove_surface_knots_report(
    source: &Surface, axis: Axis, knots: Vec<f64>, max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<SurfaceEdit> {
    source.validate()?;
    check(max_error.is_finite() && max_error > 0.,
        "Batch surface knot removal needs a positive finite error budget")?;
    let (degree, original, periodic) = match axis {
        Axis::U => (source.degree_u, &source.knots_u, source.periodic_u),
        Axis::V => (source.degree_v, &source.knots_v, source.periodic_v),
    };
    check(!periodic && knots.len() < original.len() && knots.len() >= 2*(degree+1)
        && knots.iter().all(|x| x.is_finite())
        && knots.windows(2).all(|w| w[0] <= w[1]),
        "Need a smaller non-periodic knot vector")?;
    check(original[..degree+1].iter().all(|&k| k == original[0])
        && original[original.len()-degree-1..].iter().all(|&k| k == original[original.len()-1])
        && knots[..degree+1] == original[..degree+1]
        && knots[knots.len()-degree-1..] == original[original.len()-degree-1..],
        "Batch removal must preserve clamped end knots")?;
    let mut cursor = 0;
    for &k in &knots {
        while cursor < original.len() && original[cursor] < k { cursor += 1; }
        check(cursor < original.len() && original[cursor] == k,
            "Target knots must be a multiplicity subset of original knots")?;
        cursor += 1;
    }
    let candidate = source.edit_axis(axis, |c| refit_curve(c, degree, knots.clone()))?;
    let error = crate::foundation::surface_deviation::positional_upper(source, &candidate)?;
    let accepted = error <= max_error;
    Ok(SurfaceEdit {
        surface: if accepted { candidate } else { source.clone() },
        certificate: EditCertificate {
            operation: EditOperation::SurfaceKnotRemoval,
            accepted, exact_zero_recognized: false, error_upper: error,
            budget: max_error, tolerance: context(tolerance), rebuild: None,
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

/// Indices of interior knot occurrences (with multiplicity) of a clamped vector.
fn interior_occurrences(knots: &[f64], degree: usize) -> Vec<usize> {
    let n = knots.len();
    if n < 2 * (degree + 1) {
        return Vec::new();
    }
    let (lo, hi) = (knots[degree], knots[n - degree - 1]);
    (degree + 1..n - degree - 1)
        .filter(|&i| knots[i] > lo && knots[i] < hi)
        .collect()
}

/// Greedy search shared by curve and surface removal: repeatedly try to drop
/// one interior knot occurrence; `trial` refits from the ORIGINAL geometry and
/// returns `Some((candidate, bound))` only when the bound meets the budget.
fn greedy_remove<T>(
    mut knots: Vec<f64>,
    degree: usize,
    mut trial: impl FnMut(Vec<f64>) -> Result<Option<(T, f64)>>,
) -> Result<(Option<(T, f64)>, Vec<f64>)> {
    let mut best = None;
    let mut removed = Vec::new();
    loop {
        let mut progressed = false;
        let mut index = knots.len();
        while index > 0 {
            index -= 1;
            if !interior_occurrences(&knots, degree).contains(&index) {
                continue;
            }
            let mut reduced = knots.clone();
            let knot = reduced.remove(index);
            if let Some(found) = trial(reduced.clone())? {
                knots = reduced;
                best = Some(found);
                removed.push(knot);
                progressed = true;
            }
        }
        if !progressed {
            break;
        }
    }
    Ok((best, removed))
}

fn greedy_certificate(
    operation: EditOperation,
    accepted: bool,
    error: f64,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> EditCertificate {
    EditCertificate {
        operation,
        accepted,
        exact_zero_recognized: accepted && error == 0.,
        error_upper: error,
        budget: max_error,
        tolerance: context(tolerance),
        rebuild: None,
    }
}

/// Tiller-style greedy knot removal for a non-periodic curve: removes as many
/// interior knot occurrences as the budget allows. Every trial candidate is
/// refit from the ORIGINAL curve onto the reduced knot vector and admitted
/// only by a fresh continuous deviation bound against the ORIGINAL curve, so
/// errors never accumulate across removals. Returns the edit and the removed
/// knot values; with nothing removed the source is returned with bound zero.
pub fn remove_curve_knots_report(
    source: &Curve,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<(CurveEdit, Vec<f64>)> {
    source.validate()?;
    check(!source.periodic, "Certified removal currently requires non-periodic storage")?;
    check(max_error.is_finite() && max_error >= 0., "Maximum error must be finite and nonnegative")?;
    let (best, removed) = greedy_remove(source.knots.clone(), source.degree, |knots| {
        let Ok(candidate) = refit_curve(source, source.degree, knots) else {
            return Ok(None);
        };
        let report = crate::distance::curve_deviation::inspect(
            source,
            &candidate,
            (max_error * 0.01).max(1e-12),
            4096,
        )?;
        let bound = report.bounds[1];
        Ok((bound.is_finite() && bound <= max_error).then_some((candidate, bound)))
    })?;
    let accepted = best.is_some();
    let (curve, error) = best.unwrap_or_else(|| (source.clone(), 0.));
    Ok((
        CurveEdit {
            curve,
            certificate: greedy_certificate(EditOperation::KnotRemoval, accepted, error, max_error, tolerance),
        },
        removed,
    ))
}

/// Greedy knot removal along one non-periodic surface axis. Each trial is a
/// [`remove_surface_knots_report`] refit from the ORIGINAL surface, admitted
/// only by its whole-surface positional bound against the ORIGINAL surface.
pub fn remove_surface_axis_knots_report(
    source: &Surface,
    axis: Axis,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<(SurfaceEdit, Vec<f64>)> {
    source.validate()?;
    check(max_error.is_finite() && max_error > 0., "Greedy surface knot removal needs a positive finite error budget")?;
    let (degree, original) = match axis {
        Axis::U => (source.degree_u, source.knots_u.clone()),
        Axis::V => (source.degree_v, source.knots_v.clone()),
    };
    let (best, removed) = greedy_remove(original, degree, |knots| {
        let Ok(edit) = remove_surface_knots_report(source, axis, knots, max_error, None) else {
            return Ok(None);
        };
        Ok(edit
            .certificate
            .accepted
            .then_some((edit.surface, edit.certificate.error_upper)))
    })?;
    let accepted = best.is_some();
    let (surface, error) = best.unwrap_or_else(|| (source.clone(), 0.));
    Ok((
        SurfaceEdit {
            surface,
            certificate: greedy_certificate(EditOperation::SurfaceKnotRemoval, accepted, error, max_error, tolerance),
        },
        removed,
    ))
}

/// Greedy knot removal in both surface directions. The U pass may spend the
/// whole budget; the V pass gets what remains and is bounded against the U
/// result. Both refits keep the parameterization, so the reported bound is
/// the sum of the two positional bounds (triangle inequality per parameter).
pub fn remove_surface_knots_greedy_report(
    source: &Surface,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<(SurfaceEdit, [Vec<f64>; 2])> {
    let (u, removed_u) = remove_surface_axis_knots_report(source, Axis::U, max_error, None)?;
    let spent = u.certificate.error_upper;
    let remaining = max_error - spent;
    let (surface, removed_v, v_error) = if remaining > 0. {
        let (v, removed_v) = remove_surface_axis_knots_report(&u.surface, Axis::V, remaining, None)?;
        let e = v.certificate.error_upper;
        (v.surface, removed_v, e)
    } else {
        (u.surface, Vec::new(), 0.)
    };
    let accepted = !(removed_u.is_empty() && removed_v.is_empty());
    let error = spent + v_error;
    Ok((
        SurfaceEdit {
            surface,
            certificate: greedy_certificate(EditOperation::SurfaceKnotRemoval, accepted, error, max_error, tolerance),
        },
        [removed_u, removed_v],
    ))
}
