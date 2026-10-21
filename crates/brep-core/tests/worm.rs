use brep_core::worm::{BuildLimits, finite_with_certificates as construct};
use nurbs_core::worm::{Hand, Spec};
#[test]
fn finite_za_worm_binds_original_trimmed_walls_caps_and_matches_analytic_volume() {
    for (hand, starts, turns, slab) in [
        (Hand::Right, 1, 2, [4., 5.]),
        (Hand::Left, 1, 3, [4., 8.]),
        (Hand::Right, 2, 3, [7., 14.]),
    ] {
        let spec = Spec {
            turns,
            starts,
            hand,
            ..Spec::default()
        };
        let owned = construct(spec, slab, BuildLimits::default()).unwrap();
        let model = owned.model();
        assert_eq!(model.validate().unwrap().boundary_edge_count, 0);
        assert_eq!(model.bodies.len(), 1);
        assert!(model.shells[0].closed);
        assert_eq!(owned.caps().len(), 2);
        assert!(owned.boundary_agreement().complete);
        assert_eq!(
            owned.original_wall_indices().len(),
            owned.source().trims().candidates.len()
        );
        assert!(owned.source().source().certificate().error_upper <= spec.error_budget);
        assert!(!owned.solid_geometry_certified());
        assert_eq!(owned.ideal_finite_body_error_upper(), None);
        let m = spec.axial_module;
        let p = std::f64::consts::PI * m;
        let rc = spec.diameter_quotient * m / 2. + m;
        let rr = spec.diameter_quotient * m / 2. - spec.dedendum_factor * m;
        let wc = p / 2. - spec.backlash - 2. * m * spec.pressure_angle.tan();
        let wr = p / 2. - spec.backlash + 2. * m * spec.dedendum_factor * spec.pressure_angle.tan();
        let ideal = (slab[1] - slab[0])
            * std::f64::consts::PI
            * (wc * rc * rc + (p - wr) * rr * rr + (wr - wc) * (rc * rc + rc * rr + rr * rr) / 3.)
            / p;
        assert!((owned.mass().signed_volume_mm3 - ideal).abs() < ideal * 0.005);
        for cap in owned.caps() {
            brep_core::profile_certificate::verify(
                cap.wires(),
                BuildLimits::default().tolerance_mm,
                cap.certificate(),
            )
            .unwrap();
        }
    }
}
#[test]
fn invalid_work_and_incomplete_axial_coverage_do_not_authorize_a_finite_worm() {
    let spec = Spec {
        turns: 2,
        ..Spec::default()
    };
    assert!(
        construct(
            spec,
            [4., 5.],
            BuildLimits {
                agreement_cells: 0,
                ..BuildLimits::default()
            }
        )
        .is_err()
    );
    assert!(construct(spec, [5., 4.], BuildLimits::default()).is_err());
    assert!(construct(Spec::default(), [0.5, 1.], BuildLimits::default()).is_err());
}
