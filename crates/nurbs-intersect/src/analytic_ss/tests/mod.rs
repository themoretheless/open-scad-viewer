use super::*;
use crate::coverage_verifier::{verify_complete_report, verify_incomplete_refusal};

#[test]
fn perpendicular_plane_cylinder_is_complete_circle() {
    let report = plane_cylinder(
        Plane {
            normal: [0., 0., 1.],
            offset: 2.,
        },
        [0., 0., 0.],
        [0., 0., 1.],
        3.,
        [0., 5.],
        Options::default(),
    )
    .unwrap();
    assert_eq!(report.coverage, Coverage::Complete);
    assert!(!report.permits_topology_change());
    verify_complete_report(&report, false).unwrap();
    match &report.components[0] {
        AnalyticSsComponent::Circle { radius, center, .. } => {
            assert!((radius - 3.).abs() < 1e-12);
            assert!((center[2] - 2.).abs() < 1e-12);
        }
        other => panic!("expected circle, got {other:?}"),
    }
}

#[test]
fn parallel_miss_plane_cylinder_is_complete_empty() {
    let report = plane_cylinder(
        Plane {
            normal: [1., 0., 0.],
            offset: 10.,
        },
        [0., 0., 0.],
        [0., 0., 1.],
        2.,
        [0., 4.],
        Options::default(),
    )
    .unwrap();
    assert_eq!(report.coverage, Coverage::Complete);
    verify_complete_report(&report, true).unwrap();
    assert!(matches!(report.components[0], AnalyticSsComponent::Empty));
}

#[test]
fn plane_sphere_equator_is_complete_circle() {
    let report = plane_sphere(
        Plane {
            normal: [0., 0., 1.],
            offset: 0.,
        },
        [0., 0., 0.],
        5.,
        Options::default(),
    )
    .unwrap();
    verify_complete_report(&report, false).unwrap();
    match &report.components[0] {
        AnalyticSsComponent::Circle { radius, .. } => assert!((radius - 5.).abs() < 1e-12),
        other => panic!("{other:?}"),
    }
}

#[test]
fn skewed_cylinders_refuse_out_of_matrix() {
    let report = cylinder_cylinder(
        FiniteCylinder {
            origin: [0., 0., 0.],
            direction: [0., 0., 1.],
            radius: 1.,
            height: [0., 2.],
        },
        FiniteCylinder {
            origin: [0., 0., 0.],
            direction: [1., 0., 0.],
            radius: 1.,
            height: [0., 2.],
        },
        Options::default(),
    )
    .unwrap();
    verify_incomplete_refusal(&report).unwrap();
}

#[test]
fn line_plane_miss_is_complete_empty() {
    let report = complete_line_plane(
        [0., 0., 1.],
        [0., 0., 2.],
        Plane {
            normal: [0., 0., 1.],
            offset: 0.,
        },
        Options::default(),
    )
    .unwrap();
    assert_eq!(report.coverage, Coverage::Complete);
    assert!(report.components.is_empty());
    verify_complete_report(&report, true).unwrap();
}

#[test]
fn sphere_sphere_transverse_circle_is_complete() {
    let report = sphere_sphere([0., 0., 0.], 5., [6., 0., 0.], 5., Options::default()).unwrap();
    verify_complete_report(&report, false).unwrap();
    match &report.components[0] {
        AnalyticSsComponent::Circle { radius, center, .. } => {
            assert!((radius - 4.).abs() < 1e-9);
            assert!((center[0] - 3.).abs() < 1e-9);
        }
        other => panic!("{other:?}"),
    }
}

#[test]
fn plane_torus_equator_is_complete_circle() {
    let report = plane_torus(
        Plane {
            normal: [0., 0., 1.],
            offset: 0.,
        },
        [0., 0., 0.],
        [0., 0., 1.],
        4.,
        1.,
        Options::default(),
    )
    .unwrap();
    verify_complete_report(&report, false).unwrap();
    match &report.components[0] {
        AnalyticSsComponent::Circle { radius, .. } => assert!((radius - 4.).abs() < 1e-12),
        other => panic!("{other:?}"),
    }
}

#[test]
fn cylinder_sphere_far_is_complete_empty() {
    let report = cylinder_sphere(
        [0., 0., 0.],
        [0., 0., 1.],
        1.,
        [0., 2.],
        [20., 0., 1.],
        2.,
        Options::default(),
    )
    .unwrap();
    verify_complete_report(&report, true).unwrap();
}

fn z_plane(offset: f64) -> Plane {
    Plane {
        normal: [0., 0., 1.],
        offset,
    }
}
fn tangent_point(report: &Report<AnalyticSsComponent>) -> [f64; 3] {
    verify_complete_report(report, false).unwrap();
    match report.components.as_slice() {
        [AnalyticSsComponent::Point { point, contact }] => {
            assert_eq!(*contact, Contact::Tangent { multiplicity: 2 });
            assert_eq!(contact.multiplicity(), 2);
            *point
        }
        other => panic!("expected one tangent point, got {other:?}"),
    }
}
fn cyl(origin: [f64; 3], radius: f64) -> FiniteCylinder {
    FiniteCylinder {
        origin,
        direction: [0., 0., 1.],
        radius,
        height: [0., 2.],
    }
}

#[test]
fn plane_sphere_exact_tangency_is_certified_tangent_point() {
    let report = plane_sphere(z_plane(5.), [0., 0., 0.], 5., Options::default()).unwrap();
    let p = tangent_point(&report);
    assert!((p[2] - 5.).abs() < 1e-12);
    // Unnormalised normal: the discriminant is formed without dividing.
    let report = plane_sphere(
        Plane {
            normal: [0., 0., 3.],
            offset: -6.,
        },
        [0., 0., 0.],
        2.,
        Options::default(),
    )
    .unwrap();
    let p = tangent_point(&report);
    assert!((p[2] + 2.).abs() < 1e-12);
}

#[test]
fn plane_sphere_near_tangency_is_decided_by_exact_sign() {
    // Inside the old distance band, but provably apart or provably cutting.
    let apart = 5. + 4. * f64::EPSILON;
    let report = plane_sphere(z_plane(apart), [0., 0., 0.], 5., Options::default()).unwrap();
    verify_complete_report(&report, true).unwrap();
    assert!(matches!(report.components[0], AnalyticSsComponent::Empty));
    let cutting = 5. - 4. * f64::EPSILON;
    let report = plane_sphere(z_plane(cutting), [0., 0., 0.], 5., Options::default()).unwrap();
    verify_complete_report(&report, false).unwrap();
    match report.components[0] {
        AnalyticSsComponent::Circle { radius, .. } => assert!(radius > 0. && radius < 1e-6),
        ref other => panic!("{other:?}"),
    }
}

#[test]
fn plane_sphere_overflowing_discriminant_refuses_typed() {
    let report = plane_sphere(z_plane(1e300), [0., 0., 0.], 1e300, Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(
        report.unresolved[0].reason,
        UnresolvedReason::TangencyOrMultipleRoot
    );
}

#[test]
fn sphere_sphere_external_and_internal_tangency_are_certified() {
    let report = sphere_sphere([0., 0., 0.], 2., [5., 0., 0.], 3., Options::default()).unwrap();
    let p = tangent_point(&report);
    assert!((p[0] - 2.).abs() < 1e-12);
    // Small `a` inside large `b`: contact on the far side of `a`.
    let report = sphere_sphere([0., 0., 0.], 1., [2., 0., 0.], 3., Options::default()).unwrap();
    let p = tangent_point(&report);
    assert!((p[0] + 1.).abs() < 1e-12);
    // Large `a` around small `b`.
    let report = sphere_sphere([0., 0., 0.], 3., [2., 0., 0.], 1., Options::default()).unwrap();
    let p = tangent_point(&report);
    assert!((p[0] - 3.).abs() < 1e-12);
}

#[test]
fn sphere_sphere_near_tangency_and_degenerate_centres() {
    let gap = 5. + 8. * f64::EPSILON;
    let report = sphere_sphere([0., 0., 0.], 2., [gap, 0., 0.], 3., Options::default()).unwrap();
    verify_complete_report(&report, true).unwrap();
    assert!(matches!(report.components[0], AnalyticSsComponent::Empty));
    let overlap = 5. - 8. * f64::EPSILON;
    let report =
        sphere_sphere([0., 0., 0.], 2., [overlap, 0., 0.], 3., Options::default()).unwrap();
    verify_complete_report(&report, false).unwrap();
    assert!(matches!(report.components[0], AnalyticSsComponent::Circle { .. }));
    let report = sphere_sphere([1., 1., 1.], 2., [1., 1., 1.], 3., Options::default()).unwrap();
    verify_complete_report(&report, true).unwrap();
    let report = sphere_sphere([1., 1., 1.], 2., [1., 1., 1.], 2., Options::default()).unwrap();
    assert_eq!(report.coverage, Coverage::Incomplete);
    assert_eq!(report.unresolved[0].reason, UnresolvedReason::NearCoincidence);
}

#[test]
fn plane_parallel_to_cylinder_axis_tangent_generator_is_certified() {
    let report = plane_cylinder(
        Plane {
            normal: [1., 0., 0.],
            offset: 2.,
        },
        [0., 0., 0.],
        [0., 0., 1.],
        2.,
        [0., 4.],
        Options::default(),
    )
    .unwrap();
    verify_complete_report(&report, false).unwrap();
    match report.components.as_slice() {
        [AnalyticSsComponent::TangentLine {
            start,
            end,
            multiplicity,
        }] => {
            assert_eq!(*multiplicity, 2);
            assert_eq!(*start, [2., 0., 0.]);
            assert_eq!(*end, [2., 0., 4.]);
        }
        other => panic!("{other:?}"),
    }
    // Just inside the band: two provable generators, not a refusal.
    let report = plane_cylinder(
        Plane {
            normal: [1., 0., 0.],
            offset: 2. - 4. * f64::EPSILON,
        },
        [0., 0., 0.],
        [0., 0., 1.],
        2.,
        [0., 4.],
        Options::default(),
    )
    .unwrap();
    verify_complete_report(&report, false).unwrap();
    assert_eq!(report.components.len(), 2);
}

#[test]
fn nearly_parallel_plane_cylinder_refuses_typed() {
    let report = plane_cylinder(
        Plane {
            normal: [1., 0., 1e-14],
            offset: 2.,
        },
        [0., 0., 0.],
        [0., 0., 1.],
        2.,
        [0., 4.],
        Options::default(),
    )
    .unwrap();
    verify_incomplete_refusal(&report).unwrap();
    assert_eq!(report.unresolved[0].reason, UnresolvedReason::NearCoincidence);
}

#[test]
fn parallel_cylinders_tangency_is_certified_tangent_line() {
    let report = cylinder_cylinder(cyl([0., 0., 0.], 1.), cyl([3., 0., 0.], 2.), Options::default())
        .unwrap();
    verify_complete_report(&report, false).unwrap();
    match report.components.as_slice() {
        [AnalyticSsComponent::TangentLine { start, end, .. }] => {
            assert!((start[0] - 1.).abs() < 1e-12 && start[2] == 0.);
            assert!((end[0] - 1.).abs() < 1e-12 && end[2] == 2.);
        }
        other => panic!("{other:?}"),
    }
    let report = cylinder_cylinder(cyl([0., 0., 0.], 1.), cyl([1., 0., 0.], 2.), Options::default())
        .unwrap();
    verify_complete_report(&report, false).unwrap();
    match report.components.as_slice() {
        [AnalyticSsComponent::TangentLine { start, .. }] => assert!((start[0] + 1.).abs() < 1e-12),
        other => panic!("{other:?}"),
    }
}

#[test]
fn parallel_cylinders_near_tangency_is_decided_by_exact_sign() {
    let gap = 3. + 8. * f64::EPSILON;
    let report = cylinder_cylinder(cyl([0., 0., 0.], 1.), cyl([gap, 0., 0.], 2.), Options::default())
        .unwrap();
    verify_complete_report(&report, true).unwrap();
    assert!(matches!(report.components[0], AnalyticSsComponent::Empty));
    let overlap = 3. - 8. * f64::EPSILON;
    let report = cylinder_cylinder(
        cyl([0., 0., 0.], 1.),
        cyl([overlap, 0., 0.], 2.),
        Options::default(),
    )
    .unwrap();
    verify_complete_report(&report, false).unwrap();
    assert_eq!(report.components.len(), 2);
    assert!(
        report
            .components
            .iter()
            .all(|c| matches!(c, AnalyticSsComponent::Line { .. }))
    );
}

#[test]
fn nearly_parallel_cylinders_refuse_out_of_matrix() {
    let mut b = cyl([3., 0., 0.], 2.);
    b.direction = [1e-17, 0., 1.];
    let report = cylinder_cylinder(cyl([0., 0., 0.], 1.), b, Options::default()).unwrap();
    verify_incomplete_refusal(&report).unwrap();
    assert_eq!(report.unresolved[0].reason, UnresolvedReason::UnsupportedSurface);
}
