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
