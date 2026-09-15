//! Independent coverage checks beside intersection reports.
//!
//! A `Complete` report must pass these checks before any topology consumer may
//! treat it as an imprint authority. Failure is a typed refusal, never a silent
//! downgrade to NumericallyResolved.

use crate::intersections::{Coverage, Report, UnresolvedReason};
use nurbs_core::{Error, Result};

fn refuse(message: &str) -> Error {
    Error::new("BREP_COVERAGE_VERIFIER_REFUSED", message)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CoverageAudit {
    pub complete: bool,
    pub component_count: usize,
    pub unresolved_count: usize,
    pub notes: Vec<&'static str>,
}

/// Verify that a report claiming `Complete` has an empty unresolved set and at
/// least a domain/boundary stratum story (components and/or explicit empty).
pub fn verify_complete_report<T>(report: &Report<T>, allow_empty: bool) -> Result<CoverageAudit> {
    let mut notes = Vec::new();
    if report.coverage != Coverage::Complete {
        return Err(refuse(
            "Coverage verifier requires Coverage::Complete; NumericallyResolved is not certified",
        ));
    }
    if !report.unresolved.is_empty() {
        return Err(refuse(
            "Complete report must not retain unresolved parameter boxes",
        ));
    }
    if report.components.is_empty() {
        if allow_empty {
            notes.push("empty_intersection_admitted");
        } else {
            return Err(refuse(
                "Complete report has no components and empty intersection was not admitted",
            ));
        }
    }
    Ok(CoverageAudit {
        complete: true,
        component_count: report.components.len(),
        unresolved_count: 0,
        notes,
    })
}

/// Verify Complete reports and require either components or admitted empty.
/// When `require_strata` is true, nonempty reports must record boxes_visited.
pub fn verify_complete_with_strata<T>(
    report: &Report<T>,
    allow_empty: bool,
    require_strata: bool,
) -> Result<CoverageAudit> {
    let mut audit = verify_complete_report(report, allow_empty)?;
    if require_strata && !report.components.is_empty() && report.boxes_visited == 0 {
        return Err(refuse(
            "Complete nonempty report must record strata search work (boxes_visited)",
        ));
    }
    if require_strata && !report.components.is_empty() {
        audit.notes.push("strata_work_recorded");
    }
    Ok(audit)
}

/// Negative matrix: Incomplete reports must name a typed reason, never pretend Complete.
pub fn verify_incomplete_refusal<T>(report: &Report<T>) -> Result<CoverageAudit> {
    if report.coverage == Coverage::Complete {
        return Err(refuse(
            "Incomplete refusal path must not publish Coverage::Complete",
        ));
    }
    if report.coverage == Coverage::Incomplete && report.unresolved.is_empty() {
        return Err(refuse(
            "Incomplete coverage requires at least one unresolved box with a typed reason",
        ));
    }
    for pending in &report.unresolved {
        let _ = match pending.reason {
            UnresolvedReason::BudgetExceeded
            | UnresolvedReason::TangencyOrMultipleRoot
            | UnresolvedReason::NearCoincidence
            | UnresolvedReason::BoundaryCrossing
            | UnresolvedReason::UnsupportedSurface
            | UnresolvedReason::CoincidentTrim => {}
        };
        if pending.parameter_box.is_empty() {
            return Err(refuse("Unresolved entry missing parameter box"));
        }
    }
    Ok(CoverageAudit {
        complete: false,
        component_count: report.components.len(),
        unresolved_count: report.unresolved.len(),
        notes: vec!["typed_refusal_ok"],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analytic_ss::complete_line_plane;
    use crate::intersections::{Options, Plane, Unresolved, surface_surface};
    use nurbs_core::surface::Surface;

    fn plane_surface(z: f64) -> Surface {
        Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., z], vec![0., 1., z]],
                vec![vec![1., 0., z], vec![1., 1., z]],
            ],
            weights: vec![vec![1., 1.]; 2],
            periodic_u: false,
            periodic_v: false,
        }
    }

    #[test]
    fn affine_surface_pair_promotes_to_complete_and_verifies() {
        let a = plane_surface(0.);
        let b = plane_surface(0.);
        let report = surface_surface(&a, &b, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::Complete);
        assert!(!report.permits_topology_change());
        let audit = verify_complete_report(&report, true).unwrap();
        assert!(audit.complete);
    }

    #[test]
    fn unsupported_curved_surface_is_typed_incomplete() {
        let flat = plane_surface(0.);
        let mut quadratic = flat.clone();
        quadratic.degree_u = 2;
        quadratic.degree_v = 1;
        quadratic.knots_u = vec![0., 0., 0., 1., 1., 1.];
        quadratic.knots_v = vec![0., 0., 1., 1.];
        quadratic.control_points = vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.5, 0., 1.], vec![0.5, 1., 1.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ];
        quadratic.weights = vec![vec![1., 1.], vec![1., 1.], vec![1., 1.]];
        let report = surface_surface(&flat, &quadratic, Options::default()).unwrap();
        assert_eq!(report.coverage, Coverage::Incomplete);
        verify_incomplete_refusal(&report).unwrap();
        assert!(verify_complete_report(&report, true).is_err());
    }

    #[test]
    fn complete_claim_with_unresolved_is_rejected() {
        let mut report = Report::<()>::default();
        report.coverage = Coverage::Complete;
        report.unresolved.push(Unresolved {
            parameter_box: vec![0., 1.],
            reason: UnresolvedReason::NearCoincidence,
        });
        assert!(verify_complete_report(&report, true).is_err());
    }

    #[test]
    fn line_plane_transverse_can_complete_via_analytic_helper() {
        let report = complete_line_plane(
            [0., 0., -1.],
            [0., 0., 1.],
            Plane {
                normal: [0., 0., 1.],
                offset: 0.,
            },
            Options::default(),
        )
        .unwrap();
        assert_eq!(report.coverage, Coverage::Complete);
        verify_complete_report(&report, false).unwrap();
    }
}
