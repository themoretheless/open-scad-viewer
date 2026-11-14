//! Certified tangential curve/plane contacts (roadmap N1, first slice).
use super::*;

/// 120-degree arc of the unit circle in the xz-plane, apex (0,0,1) at t=0.5.
fn arc() -> Curve {
    let s = 3f64.sqrt() * 0.5;
    Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![-s, 0., 0.5], vec![0., 0., 2.], vec![s, 0., 0.5]],
        weights: vec![1., 0.5, 1.],
        periodic: false,
    }
}
fn roots(report: &Report<CurvePlaneComponent>) -> Vec<(f64, u32)> {
    points(report)
        .iter()
        .map(|p| (p.parameter, p.contact.multiplicity()))
        .collect()
}
fn refused(report: &Report<CurvePlaneComponent>) -> bool {
    report.coverage == Coverage::Incomplete
        && report
            .unresolved
            .iter()
            .any(|u| u.reason == UnresolvedReason::TangencyOrMultipleRoot)
}

#[test]
fn rational_circle_tangent_to_plane_is_an_even_multiplicity_point() {
    let report = curve_plane(&arc(), plane(1.), Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert!(report.unresolved.is_empty());
    assert_eq!(roots(&report), vec![(0.5, 2)], "{report:?}");
    let p = points(&report)[0];
    assert_eq!(p.contact, Contact::Tangent { multiplicity: 2 });
    assert!(p.plane_residual < 1e-15 && p.point[0].abs() < 1e-15);
}

#[test]
fn near_tangency_is_certified_as_zero_or_two_transverse_roots() {
    for delta in [1e-3, 1e-6, 1e-9] {
        let above = curve_plane(&arc(), plane(1. + delta), Options::default()).unwrap();
        assert!(above.components.is_empty(), "{delta}: {above:?}");
        assert_eq!(above.coverage, Coverage::NumericallyResolved, "{delta}: {above:?}");

        let below = curve_plane(&arc(), plane(1. - delta), Options::default()).unwrap();
        assert_eq!(below.coverage, Coverage::NumericallyResolved, "{delta}: {below:?}");
        let found = points(&below);
        assert_eq!(found.len(), 2, "{delta}: {below:?}");
        for root in &found {
            assert_eq!(root.contact.multiplicity(), 1, "{delta}: {below:?}");
            assert!(root.plane_residual <= 1e-9, "{delta}: {below:?}");
        }
        // Symmetric about the apex and separated by ~sqrt(delta).
        assert!((found[0].parameter + found[1].parameter - 1.).abs() < 1e-9);
        assert!(found[1].parameter - found[0].parameter > delta.sqrt() * 0.5);
    }
}

#[test]
fn cubic_double_root_is_tangent_beside_a_transverse_root() {
    // 3 (t-1/2)^2 (t-3/4)
    let report = curve_plane(
        &bezier(&[-0.5625, 0.4375, -0.3125, 0.1875]),
        plane(0.),
        Options::default(),
    )
    .unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    let found = roots(&report);
    assert_eq!(found.len(), 2, "{report:?}");
    assert_eq!(found[0], (0.5, 2));
    assert!((found[1].0 - 0.75).abs() < 1e-9 && found[1].1 == 1);
    assert_eq!(points(&report)[1].contact, Contact::Transverse);
}

#[test]
fn triple_root_crosses_with_multiplicity_three() {
    // (t-1/2)^3
    let report = curve_plane(
        &bezier(&[-0.125, 0.125, -0.125, 0.125]),
        plane(0.),
        Options::default(),
    )
    .unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert_eq!(roots(&report), vec![(0.5, 3)]);
}

#[test]
fn quadratic_double_root_replaces_the_old_refusal() {
    let report =
        curve_plane(&bezier(&[0.25, -0.25, 0.25]), plane(0.), Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved, "{report:?}");
    assert_eq!(roots(&report), vec![(0.5, 2)]);
}

#[test]
fn tangency_at_a_non_dyadic_parameter_stays_a_typed_refusal() {
    // (1-3t)^2: exact double root at t = 1/3, which no binary64 bisection
    // point can hit, so the contact cannot be certified and is refused.
    let report = curve_plane(&bezier(&[1., -2., 4.]), plane(0.), Options::default()).unwrap();
    assert!(refused(&report), "{report:?}");
    assert!(points(&report).iter().all(|p| p.contact.multiplicity() == 1));
}

#[test]
fn curve_in_plane_is_an_overlap_interval() {
    let report = curve_plane(&bezier(&[0., 0., 0.]), plane(0.), Options::default()).unwrap();
    assert_eq!(report.components.len(), 1);
    assert!(matches!(
        report.components[0],
        CurvePlaneComponent::Overlap {
            parameter_interval: [0., 1.],
            ..
        }
    ));
    // The rational arc lies in the plane y = 0.
    let report = curve_plane(
        &arc(),
        Plane {
            normal: [0., 1., 0.],
            offset: 0.,
        },
        Options::default(),
    )
    .unwrap();
    assert!(matches!(
        report.components[..],
        [CurvePlaneComponent::Overlap { .. }]
    ));
}

#[test]
fn transverse_roots_keep_multiplicity_one() {
    let report = curve_plane(
        &bezier(&[-0.08, 0.14, -0.14, 0.08]),
        plane(0.),
        Options::default(),
    )
    .unwrap();
    assert_eq!(report.coverage, Coverage::NumericallyResolved);
    assert_eq!(points(&report).len(), 3);
    assert!(points(&report).iter().all(|p| p.contact.multiplicity() == 1));
}
