//! Knot-removal feasibility pre-check (item 374).
//!
//! `assess_knot_removal` projects the control movement a removal would force
//! (the reverse of Boehm insertion: refit without the knot, reinsert, and
//! measure the net displacement) and certifies a deviation bound BEFORE any
//! edit is committed. It pairs with
//! `approximate_edits::remove_curve_knot_report`, which performs the actual
//! certified removal given a tolerance budget.
use crate::foundation::guards::{Budget, require_finite_f64};
use crate::{Result, check, curve::Curve};

/// Pre-check result for removing one interior knot.
pub struct KnotRemovalAssessment {
    /// The assessed knot parameter.
    pub knot: f64,
    /// The tolerance at which removal provably holds: zero when the removal
    /// is exact at the control-net level (the Boehm projection vanishes to
    /// rounding), otherwise the fixed point of the certified deviation bound
    /// under `remove_curve_knot_report`'s budget-dependent accuracy (1% of
    /// the budget). Acceptance is monotone in the budget above this value.
    pub removable_at_tolerance: f64,
    /// Boehm-projection norm: maximum Euclidean distance between the source
    /// control net and the refit-then-reinserted net — the predicted control
    /// movement the removal forces. Vanishes (to rounding) iff the knot is
    /// exactly removable.
    pub control_delta_norm: f64,
    /// True when the removal pipeline produced a finite certified bound and
    /// `remove_curve_knot_report` accepts at the certified envelope budget.
    pub feasible: bool,
}

/// Assesses removal of one interior knot BEFORE operating. `max_passes` is
/// the deviation-certification cell budget (0 selects the default of 4096).
pub fn assess_knot_removal(
    curve: &Curve,
    knot: f64,
    max_passes: usize,
) -> Result<KnotRemovalAssessment> {
    curve.validate()?;
    check(
        !curve.periodic,
        "Knot-removal assessment currently requires non-periodic storage",
    )?;
    let domain = curve.domain();
    require_finite_f64(knot, "knot_removal_knot")?;
    check(
        knot > domain[0] && knot < domain[1],
        "Only interior knots can be assessed",
    )?;
    let Some(index) = curve.knots.iter().position(|&value| value == knot) else {
        return Err(crate::input("Knot is absent"));
    };
    let mut knots = curve.knots.clone();
    knots.remove(index);
    let candidate = super::refit_curve(curve, curve.degree, knots)?;
    // Boehm-projection norm: reinsertion restores the control count, so the
    // nets compare index-by-index. Knot insertion is the exact inverse of an
    // exact removal, hence this norm vanishes iff the knot is removable
    // exactly.
    let reconstructed = candidate.insert(knot, 1)?;
    let control_delta_norm = curve
        .control_points
        .iter()
        .zip(&reconstructed.control_points)
        .map(|(a, b)| {
            a.iter()
                .zip(b)
                .map(|(x, y)| (x - y) * (x - y))
                .sum::<f64>()
                .sqrt()
        })
        .fold(0., f64::max);
    let scale = curve
        .control_points
        .iter()
        .flatten()
        .map(|x| x.abs())
        .fold(1., f64::max);
    let exact_removal = control_delta_norm <= 1e-9 * scale;
    // Fixed point of the certified bound under remove_curve_knot_report's
    // accuracy rule (max_error * 0.01, floored at 1e-12): iterating makes the
    // assessed budget self-consistent, so the paired removal accepts exactly
    // at the reported bound.
    let budget = if max_passes == 0 { 4096 } else { max_passes };
    let mut accuracy = 1e-12;
    let mut bound = crate::curve_deviation::inspect(curve, &candidate, accuracy, budget)?
        .bounds[1];
    // Fixed-point iteration on the certified bound, under a unified budget
    // guard (8 passes, matching the historical cap).
    let mut fixpoint = Budget::with_iterations(8)?.guard("knot-removal-bound-fixpoint");
    for _ in 0..8 {
        fixpoint.tick()?;
        let next_accuracy = (bound * 0.01).max(1e-12);
        if next_accuracy == accuracy {
            break;
        }
        accuracy = next_accuracy;
        let next = crate::curve_deviation::inspect(curve, &candidate, accuracy, budget)?
            .bounds[1];
        let stable = (next - bound).abs() <= 1e-12 * bound.max(1e-300);
        bound = next;
        if stable {
            break;
        }
    }
    crate::numeric(bound.is_finite(), "Knot-removal deviation bound overflowed")?;
    let removable_at_tolerance = if exact_removal { 0. } else { bound };
    let feasible = super::approximate_edits::remove_curve_knot_report(curve, knot, bound, None)
        .map(|edit| edit.certificate.accepted)
        .unwrap_or(false);
    Ok(KnotRemovalAssessment {
        knot,
        removable_at_tolerance,
        control_delta_norm,
        feasible,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Clamped cubic with one single interior knot at u = 1.
    fn base_curve() -> Curve {
        let curve = Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 2., 2., 2., 2.],
            control_points: vec![
                vec![0., 0.],
                vec![1., 2.],
                vec![2., 2.],
                vec![3., 0.],
                vec![4., 0.],
            ],
            weights: vec![1.; 5],
            periodic: false,
        };
        curve.validate().unwrap();
        curve
    }

    #[test]
    fn inserted_knot_assesses_as_exactly_removable() {
        let base = base_curve();
        let refined = base.insert(0.5, 1).unwrap();
        let assessment = assess_knot_removal(&refined, 0.5, 0).unwrap();
        assert_eq!(
            assessment.removable_at_tolerance, 0.,
            "freshly inserted knot must be exactly removable"
        );
        assert!(
            assessment.control_delta_norm <= 1e-9,
            "predicted control movement must be ~zero, got {}",
            assessment.control_delta_norm
        );
        assert!(assessment.feasible);
        // Paired removal agrees: accepted once the budget clears the
        // (conservative) certified envelope floor of this curve.
        let edit =
            super::super::approximate_edits::remove_curve_knot_report(&refined, 0.5, 0.01, None)
                .unwrap();
        assert!(edit.certificate.accepted);
    }

    #[test]
    fn perturbed_control_point_requires_positive_tolerance() {
        let base = base_curve();
        let mut refined = base.insert(0.5, 1).unwrap();
        // Perturb one control point so the knot is no longer exactly removable.
        refined.control_points[2][1] += 0.1;
        refined.validate().unwrap();
        let assessment = assess_knot_removal(&refined, 0.5, 0).unwrap();
        assert!(
            assessment.removable_at_tolerance > 1e-6,
            "perturbed knot must need a positive tolerance, got {}",
            assessment.removable_at_tolerance
        );
        assert!(assessment.control_delta_norm > 1e-6);
        assert!(assessment.feasible);
        // Agreement with the paired removal: rejected well below the assessed
        // tolerance, accepted comfortably above it.
        let rejected = super::super::approximate_edits::remove_curve_knot_report(
            &refined,
            0.5,
            assessment.removable_at_tolerance * 0.5,
            None,
        )
        .unwrap();
        assert!(!rejected.certificate.accepted);
        let accepted = super::super::approximate_edits::remove_curve_knot_report(
            &refined,
            0.5,
            assessment.removable_at_tolerance * 2. + 1e-9,
            None,
        )
        .unwrap();
        assert!(accepted.certificate.accepted);
    }

    #[test]
    fn absent_and_boundary_knots_are_rejected() {
        let base = base_curve();
        assert!(assess_knot_removal(&base, 0.5, 0).is_err(), "absent knot");
        assert!(assess_knot_removal(&base, 0., 0).is_err(), "boundary knot");
        assert!(assess_knot_removal(&base, 2., 0).is_err(), "boundary knot");
        assert!(assess_knot_removal(&base, f64::NAN, 0).is_err(), "NaN knot");
    }

    #[test]
    fn non_finite_knot_is_a_typed_boundary_error() {
        let base = base_curve();
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let err = assess_knot_removal(&base, value, 0).err().expect("invalid knot removal input must fail");
            assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
            assert!(err.contains("knot_removal_knot"), "{err}");
        }
    }

    #[test]
    fn certified_bound_fixpoint_runs_under_budget() {
        // A perturbed removal exercises the fixpoint loop; it must terminate
        // within the 8-pass guard and still agree with the paired removal.
        let base = base_curve();
        let mut refined = base.insert(0.5, 1).unwrap();
        refined.control_points[2][1] += 0.05;
        refined.validate().unwrap();
        let assessment = assess_knot_removal(&refined, 0.5, 0).unwrap();
        assert!(assessment.removable_at_tolerance.is_finite());
        assert!(assessment.feasible);
    }
}

// IEEE-754 input policy tests (items 372, 376).
//
// NOTE: this module intentionally lives here even though it exercises
// crate-wide APIs (`Curve::validate`, `CurveEvaluator::evaluate`,
// `surface_offset::offset`) rather than knot removal itself — the placement
// keeps the policy pins in a single test-only module without touching files
// owned by other lines of work. Each test pins DETERMINISTIC behavior: a
// validation error, never a panic, never silent NaN propagation.
#[cfg(test)]
mod ieee_policy_tests {
    use crate::curve::{Curve, CurveEvaluator};

    /// A valid clamped cubic; tests mutate one field at a time.
    fn valid_curve() -> Curve {
        Curve {
            degree: 3,
            knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
            control_points: vec![
                vec![0., 0.],
                vec![1., 2.],
                vec![2., 2.],
                vec![3., 0.],
            ],
            weights: vec![1.; 4],
            periodic: false,
        }
    }

    fn assert_invalid(curve: &Curve, needle: &str) {
        let error = curve.validate().unwrap_err();
        assert!(
            error.to_string().contains(needle),
            "error must mention {needle:?}: {error}"
        );
    }

    #[test]
    fn nan_coordinate_is_a_deterministic_validation_error() {
        let mut curve = valid_curve();
        curve.control_points[1][0] = f64::NAN;
        assert_invalid(&curve, "finite coordinates");
    }

    #[test]
    fn infinite_coordinates_are_deterministic_validation_errors() {
        for value in [f64::INFINITY, f64::NEG_INFINITY] {
            let mut curve = valid_curve();
            curve.control_points[2][1] = value;
            assert_invalid(&curve, "finite coordinates");
        }
    }

    #[test]
    fn negative_zero_coordinate_is_accepted_and_deterministic() {
        // IEEE −0.0 is finite and compares equal to 0.0; policy: accept it.
        let mut curve = valid_curve();
        curve.control_points[0][0] = -0.0;
        curve.validate().unwrap();
        let point = curve.evaluate(0.).unwrap().point;
        assert_eq!(point[0], 0., "−0.0 evaluates as 0.0");
        assert!(point.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn nonpositive_and_nonfinite_weights_are_validation_errors() {
        for (label, weight) in [
            ("zero weight", 0.),
            ("negative weight", -1.),
            ("+∞ weight", f64::INFINITY),
            ("−∞ weight", f64::NEG_INFINITY),
            ("NaN weight", f64::NAN),
        ] {
            let mut curve = valid_curve();
            curve.weights[1] = weight;
            assert_invalid(&curve, "positive, finite");
            let _ = label;
        }
    }

    #[test]
    fn mixed_valid_and_nan_weights_are_rejected() {
        let mut curve = valid_curve();
        curve.weights = vec![1., f64::NAN, 0.5, 2.];
        assert_invalid(&curve, "positive, finite");
    }

    #[test]
    fn nan_knot_is_a_deterministic_validation_error() {
        let mut curve = valid_curve();
        curve.knots[3] = f64::NAN;
        assert_invalid(&curve, "finite");
    }

    #[test]
    fn evaluator_rejects_nonfinite_parameters_without_panicking() {
        let curve = valid_curve();
        let mut evaluator = CurveEvaluator::new(&curve).unwrap();
        for parameter in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let error = evaluator.evaluate(parameter).unwrap_err();
            assert!(
                error.to_string().contains("outside the active knot domain"),
                "parameter {parameter}: unexpected error {error}"
            );
        }
        // The evaluator stays usable after rejected parameters.
        assert!(evaluator.evaluate(0.5).unwrap().point.iter().all(|v| v.is_finite()));
    }

    #[test]
    fn evaluate_rejects_nonfinite_parameters_without_panicking() {
        let curve = valid_curve();
        for parameter in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert!(curve.evaluate(parameter).is_err());
        }
    }

    #[test]
    fn surface_offset_rejects_nonfinite_distance_and_tolerance() {
        use crate::surface::Surface;
        let plane = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., 0.], vec![0., 10., 0.]],
                vec![vec![10., 0., 0.], vec![10., 10., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        plane.validate().unwrap();
        for distance in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            let Err(error) = crate::surface_offset::offset(&plane, distance, 0.1) else {
                panic!("non-finite distance {distance} must be rejected")
            };
            assert!(
                error.to_string().contains("distance must be finite"),
                "distance {distance}: unexpected error {error}"
            );
        }
        for tolerance in [f64::NAN, f64::INFINITY, 0., -0.1] {
            assert!(
                crate::surface_offset::offset(&plane, 0.5, tolerance).is_err(),
                "tolerance {tolerance} must be rejected"
            );
        }
    }

    #[test]
    fn surface_with_nan_coordinate_fails_validation_before_offset() {
        use crate::surface::Surface;
        let mut plane = Surface {
            degree_u: 1,
            degree_v: 1,
            knots_u: vec![0., 0., 1., 1.],
            knots_v: vec![0., 0., 1., 1.],
            control_points: vec![
                vec![vec![0., 0., f64::NAN], vec![0., 10., 0.]],
                vec![vec![10., 0., 0.], vec![10., 10., 0.]],
            ],
            weights: vec![vec![1.; 2]; 2],
            periodic_u: false,
            periodic_v: false,
        };
        assert!(plane.validate().is_err(), "NaN coordinate must be rejected");
        plane.control_points[0][0][2] = f64::INFINITY;
        assert!(plane.validate().is_err(), "+∞ coordinate must be rejected");
        // …and offset therefore errors instead of propagating NaN.
        assert!(crate::surface_offset::offset(&plane, 0.5, 0.1).is_err());
    }
}
