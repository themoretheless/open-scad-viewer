//! Native periodic edits, seam continuity evidence and seam splitting.
use super::{
    Axis, Curve, Result, Surface, ToleranceContext, box_of, check, context, distance, next_up,
    periodic_active_knots, periodic_error, refit_periodic,
};
#[cfg(feature = "codec")]
pub mod serialization;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PeriodicEditOperation {
    Insert,
    Remove,
    Elevate,
    Reduce,
}
pub struct SeamContinuity {
    pub residual_upper: Option<f64>,
    pub certified: bool,
}
pub struct SeamCertificate {
    pub domain: [f64; 2],
    pub period: f64,
    pub c0: SeamContinuity,
    pub c1: SeamContinuity,
    pub c2: SeamContinuity,
}
pub struct PeriodicEditEvidence {
    pub operation: PeriodicEditOperation,
    pub accepted: bool,
    pub error_upper: f64,
    pub budget: f64,
    pub tolerance: ToleranceContext,
}
pub struct PeriodicCurveEdit {
    pub curve: Curve,
    pub seam: SeamCertificate,
    pub evidence: PeriodicEditEvidence,
}
pub struct PeriodicSurfaceEdit {
    pub surface: Surface,
    pub axis: Axis,
    pub seam: SeamCertificate,
    pub sampled_transverse_lines: usize,
    pub evidence: PeriodicEditEvidence,
}
pub struct PeriodicCurveSplit {
    pub curves: [Curve; 2],
    pub seam: SeamCertificate,
    pub coverage: [[f64; 2]; 2],
    pub tolerance: ToleranceContext,
}

pub fn seam_certificate(curve: &Curve) -> Result<SeamCertificate> {
    let [a, b] = curve.domain();
    let first = curve.evaluate(a)?;
    let last = curve.evaluate(b)?;
    let residual = |x: &[f64], y: &[f64]| next_up(distance(x, y));
    let c0 = residual(&first.point, &last.point);
    let c1 = match (&first.d1, &last.d1) {
        (Some(x), Some(y)) => Some(residual(x, y)),
        _ => None,
    };
    let c2 = match (&first.d2, &last.d2) {
        (Some(x), Some(y)) => Some(residual(x, y)),
        _ => None,
    };
    Ok(SeamCertificate {
        domain: [a, b],
        period: b - a,
        c0: SeamContinuity {
            residual_upper: Some(c0),
            certified: c0 <= 64. * f64::EPSILON,
        },
        c1: SeamContinuity {
            residual_upper: c1,
            certified: c1.is_some_and(|v| v <= 256. * f64::EPSILON),
        },
        c2: SeamContinuity {
            residual_upper: c2,
            certified: c2.is_some_and(|v| v <= 1024. * f64::EPSILON),
        },
    })
}

fn periodic_candidate(
    source: &Curve,
    operation: PeriodicEditOperation,
    parameter: f64,
    degree: usize,
) -> Result<Curve> {
    let mut active = periodic_active_knots(source);
    let target_degree = match operation {
        PeriodicEditOperation::Insert => {
            check(
                parameter >= active[0] && parameter < *active.last().unwrap(),
                "Periodic insertion must use the half-open fundamental domain",
            )?;
            active.insert(
                active.partition_point(|value| *value <= parameter),
                parameter,
            );
            source.degree
        }
        PeriodicEditOperation::Remove => {
            let index = active
                .iter()
                .position(|value| {
                    *value == parameter && *value > active[0] && *value < *active.last().unwrap()
                })
                .ok_or_else(|| crate::input("Removable periodic knot is absent"))?;
            active.remove(index);
            source.degree
        }
        PeriodicEditOperation::Elevate => {
            check(
                degree > source.degree && degree <= 25,
                "Periodic elevation target is invalid",
            )?;
            let delta = degree - source.degree;
            let original = active.clone();
            for value in original.iter().rev() {
                let index = active.partition_point(|k| *k <= *value);
                for _ in 0..delta {
                    active.insert(index, *value);
                }
            }
            degree
        }
        PeriodicEditOperation::Reduce => {
            check(
                degree >= 1 && degree < source.degree,
                "Periodic reduction target is invalid",
            )?;
            for _ in 0..source.degree - degree {
                let distinct =
                    active
                        .iter()
                        .copied()
                        .fold(Vec::<f64>::new(), |mut values, value| {
                            if values.last() != Some(&value) {
                                values.push(value)
                            }
                            values
                        });
                for value in distinct.into_iter().rev() {
                    if active.iter().filter(|k| **k == value).count() > 1 {
                        active.remove(active.iter().rposition(|k| *k == value).unwrap());
                    }
                }
            }
            degree
        }
    };
    refit_periodic(source, target_degree, active)
}

pub fn edit_periodic_curve_report(
    source: &Curve,
    operation: PeriodicEditOperation,
    parameter: f64,
    degree: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<PeriodicCurveEdit> {
    source.validate()?;
    check(
        source.periodic,
        "Wrapped periodic edit requires periodic storage",
    )?;
    check(
        max_error.is_finite() && max_error >= 0.,
        "Maximum error must be finite and nonnegative",
    )?;
    let candidate = periodic_candidate(source, operation, parameter, degree)?;
    let error = periodic_error(source, &candidate)?;
    let accepted = error <= max_error;
    let output = if accepted { candidate } else { source.clone() };
    let seam = seam_certificate(&output)?;
    let tolerance = context(tolerance);
    Ok(PeriodicCurveEdit {
        curve: output,
        seam,
        evidence: PeriodicEditEvidence {
            operation,
            accepted,
            error_upper: error,
            budget: max_error,
            tolerance,
        },
    })
}

pub fn edit_periodic_surface_report(
    source: &Surface,
    axis: Axis,
    operation: PeriodicEditOperation,
    parameter: f64,
    degree: usize,
    max_error: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<PeriodicSurfaceEdit> {
    source.validate()?;
    let periodic = match axis {
        Axis::U => source.periodic_u,
        Axis::V => source.periodic_v,
    };
    check(periodic, "Selected surface axis is not periodic")?;
    let candidate = source.edit_axis(axis, |curve| {
        periodic_candidate(curve, operation, parameter, degree)
    })?;
    let mut error = 0_f64;
    let samples = 32;
    for i in 0..=samples {
        let parameter_other = match axis {
            Axis::U => {
                source.knots_v[source.degree_v]
                    + (source.knots_v[source.control_points[0].len()]
                        - source.knots_v[source.degree_v])
                        * i as f64
                        / samples as f64
            }
            Axis::V => {
                source.knots_u[source.degree_u]
                    + (source.knots_u[source.control_points.len()]
                        - source.knots_u[source.degree_u])
                        * i as f64
                        / samples as f64
            }
        };
        let fixed = match axis {
            Axis::U => Axis::V,
            Axis::V => Axis::U,
        };
        error = error.max(periodic_error(
            &source.iso(fixed, parameter_other)?,
            &candidate.iso(fixed, parameter_other)?,
        )?);
    }
    let controls = source
        .control_points
        .iter()
        .flatten()
        .chain(candidate.control_points.iter().flatten())
        .cloned()
        .collect::<Vec<_>>();
    let (minimum, maximum) = box_of(&controls);
    error = error.max(next_up(
        minimum
            .iter()
            .zip(maximum)
            .map(|(a, b)| (b - a) * (b - a))
            .sum::<f64>()
            .sqrt(),
    ));
    let accepted = error <= max_error;
    let output = if accepted { candidate } else { source.clone() };
    let representative = match axis {
        Axis::U => output.iso(Axis::V, output.knots_v[output.degree_v])?,
        Axis::V => output.iso(Axis::U, output.knots_u[output.degree_u])?,
    };
    let tolerance = context(tolerance);
    Ok(PeriodicSurfaceEdit {
        surface: output,
        axis,
        seam: seam_certificate(&representative)?,
        sampled_transverse_lines: samples + 1,
        evidence: PeriodicEditEvidence {
            operation,
            accepted,
            error_upper: next_up(error),
            budget: max_error,
            tolerance,
        },
    })
}

pub fn split_periodic_curve_report(
    source: &Curve,
    parameter: f64,
    tolerance: Option<ToleranceContext>,
) -> Result<PeriodicCurveSplit> {
    source.validate()?;
    check(source.periodic, "Periodic split requires periodic storage")?;
    let [a, b] = source.domain();
    check(
        parameter > a && parameter < b,
        "Periodic split must be strictly inside the fundamental domain",
    )?;
    let pieces = [source.trim(parameter, b)?, source.trim(a, parameter)?];
    let tolerance = context(tolerance);
    Ok(PeriodicCurveSplit {
        curves: pieces,
        seam: seam_certificate(source)?,
        coverage: [[parameter, b], [a, parameter]],
        tolerance,
    })
}
