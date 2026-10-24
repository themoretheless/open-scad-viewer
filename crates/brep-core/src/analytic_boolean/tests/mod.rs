use super::*;
use crate::{cylinder, extrude_polygon, sphere};

#[test]
fn disjoint_cylinders_union_carries_complete_certificate() {
    let a = cylinder(2., 4.).unwrap();
    let b0 = cylinder(2., 4.).unwrap();
    let b = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 10.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let (model, cert) = analytic_boolean(&a, &b, "union").unwrap();
    assert!(cert.imprint_authored);
    assert!(cert.no_prism_authorship);
    assert!(cert.permits_topology_change());
    assert!(cert.all_complete());
    assert!(!model.bodies.is_empty());
    model.validate().unwrap();
}

#[test]
fn contained_cylinder_difference_is_certified() {
    let outer = cylinder(4., 6.).unwrap();
    let inner = cylinder(1.5, 6.).unwrap();
    let (model, cert) = analytic_boolean(&outer, &inner, "difference").unwrap();
    assert!(cert.imprint_authored);
    assert!(cert.no_prism_authorship);
    assert!(cert.all_complete());
    model.validate().unwrap();
}

#[test]
fn identical_cylinders_intersection_is_certified() {
    let a = cylinder(3., 5.).unwrap();
    let b = cylinder(3., 5.).unwrap();
    let (model, cert) = analytic_boolean(&a, &b, "intersection").unwrap();
    assert!(cert.no_prism_authorship);
    assert!(cert.all_complete());
    model.validate().unwrap();
}

#[test]
fn box_sphere_dome_is_certified_for_every_operation() {
    let b = crate::cuboid([-10., -10., -10.], [10., 10., 10.]).unwrap();
    let s = crate::transform::affine(
        &sphere(5.).unwrap(),
        [
            [1., 0., 0., 0.3],
            [0., 1., 0., -0.7],
            [0., 0., 1., 8.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    for operation in ["union", "difference", "intersection"] {
        let (model, cert) = analytic_boolean(&b, &s, operation).unwrap();
        assert!(cert.no_prism_authorship, "{operation}");
        assert!(cert.all_complete(), "{operation}");
        assert_eq!(model.bodies.len(), 1, "{operation}");
        model.validate().unwrap();
        let (swapped, _) = analytic_boolean(&s, &b, operation).unwrap();
        swapped.validate().unwrap();
    }
    let err = analytic_boolean(&b, &s, "xor").unwrap_err();
    assert_eq!(err.code, "BREP_ANALYTIC_BOOLEAN_REFUSED");
}

#[test]
fn box_sphere_edge_corner_and_bar_are_certified() {
    let place = |r: f64, at: [f64; 3]| {
        crate::transform::affine(
            &sphere(r).unwrap(),
            [
                [1., 0., 0., at[0]],
                [0., 1., 0., at[1]],
                [0., 0., 1., at[2]],
                [0., 0., 0., 1.],
            ],
        )
        .unwrap()
    };
    let big = crate::cuboid([-10., -10., -10.], [10., 10., 10.]).unwrap();
    let bar = crate::cuboid([-10., -1.5, -1.2], [10., 1.5, 1.2]).unwrap();
    let cases = [
        (big.clone(), place(4., [9., 0.4, 9.])),
        (big, place(4., [9.3, 8.9, 9.1])),
        (bar, place(4., [0.3, 0.1, -0.2])),
    ];
    for (b, s) in &cases {
        for operation in ["union", "difference", "intersection"] {
            let (model, cert) = analytic_boolean(b, s, operation).unwrap();
            assert!(cert.no_prism_authorship, "{operation}");
            assert!(cert.all_complete(), "{operation}");
            model.validate().unwrap();
        }
    }
    let (bar_cut, _) = analytic_boolean(&cases[2].0, &cases[2].1, "difference").unwrap();
    assert_eq!(bar_cut.bodies.len(), 2);
}

#[test]
fn planar_box_pair_refuses_analytic_path() {
    let a = extrude_polygon(&[[0., 0.], [2., 0.], [2., 2.], [0., 2.]], 0., 1.).unwrap();
    let b = extrude_polygon(&[[1., 1.], [3., 1.], [3., 3.], [1., 3.]], 0., 1.).unwrap();
    let err = analytic_boolean(&a, &b, "union").unwrap_err();
    assert_eq!(err.code, "BREP_UNSUPPORTED_OPERATION");
}

#[test]
fn axial_cylinder_sphere_is_certified() {
    let c = cylinder(3., 10.).unwrap();
    let s = crate::transform::affine(
        &sphere(4.).unwrap(),
        [
            [1., 0., 0., 0.],
            [0., 1., 0., 0.],
            [0., 0., 1., 5.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    for operation in ["union", "difference", "intersection"] {
        let (model, cert) = analytic_boolean(&c, &s, operation).unwrap();
        assert!(cert.no_prism_authorship, "{operation}");
        assert!(cert.all_complete(), "{operation}");
        model.validate().unwrap();
        let (swapped, _) = analytic_boolean(&s, &c, operation).unwrap();
        swapped.validate().unwrap();
    }
    let (stubs, _) = analytic_boolean(&c, &s, "difference").unwrap();
    assert_eq!(stubs.bodies.len(), 2);
}

#[test]
fn mixed_sphere_cylinder_refuses() {
    let a = cylinder(2., 4.).unwrap();
    let a_faces = a.faces.len();
    let b = sphere(2.).unwrap();
    // Overlapping mixed pair without Complete imprint → refuse (operands unchanged).
    let err = analytic_boolean(&a, &b, "union").unwrap_err();
    assert!(
        err.code == "BREP_UNSUPPORTED_OPERATION" || err.code == "BREP_ANALYTIC_BOOLEAN_REFUSED"
    );
    assert_eq!(a.faces.len(), a_faces);
}

#[test]
fn mixed_sphere_cylinder_disjoint_is_certified() {
    let a = cylinder(2., 4.).unwrap();
    let b0 = sphere(2.).unwrap();
    let b = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 20.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let (model, cert) = analytic_boolean(&a, &b, "union").unwrap();
    assert!(cert.no_prism_authorship);
    assert!(cert.all_complete());
    model.validate().unwrap();
}

#[test]
fn disjoint_cones_union_is_certified() {
    let a = crate::frustum(2., 0.5, 4.).unwrap();
    let b0 = crate::frustum(2., 0.5, 4.).unwrap();
    let b = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 20.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let (model, cert) = analytic_boolean(&a, &b, "union").unwrap();
    assert!(cert.all_complete());
    model.validate().unwrap();
}

#[test]
fn disjoint_tori_boolean_is_query_only_refuse() {
    let a = crate::torus(4., 1.).unwrap();
    let b0 = crate::torus(4., 1.).unwrap();
    let b = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 30.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let error = analytic_boolean(&a, &b, "union").unwrap_err();
    assert_eq!(error.code, "BREP_ANALYTIC_BOOLEAN_REFUSED");
}

#[test]
fn intersecting_cones_freeze_refuse() {
    let a = crate::frustum(2., 0.5, 4.).unwrap();
    let b0 = crate::frustum(2., 0.5, 4.).unwrap();
    let b = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 1.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let err = analytic_boolean(&a, &b, "union").unwrap_err();
    assert_eq!(err.code, "BREP_ANALYTIC_BOOLEAN_REFUSED");
}

#[test]
fn sphere_sphere_union_is_certified() {
    let a = sphere(2.).unwrap();
    let b0 = sphere(2.).unwrap();
    let b = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 10.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let (model, cert) = analytic_boolean(&a, &b, "union").unwrap();
    assert!(cert.no_prism_authorship);
    assert!(cert.all_complete());
    model.validate().unwrap();
}

#[test]
fn tangent_cylinders_refuse() {
    let a = cylinder(2., 4.).unwrap();
    let b0 = cylinder(2., 4.).unwrap();
    let b = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 4.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let err = analytic_boolean(&a, &b, "union").unwrap_err();
    assert_eq!(err.code, "BREP_ANALYTIC_BOOLEAN_REFUSED");
}

#[test]
fn empty_intersection_of_disjoint_cylinders_is_certified() {
    let a = cylinder(2., 4.).unwrap();
    let b0 = cylinder(2., 4.).unwrap();
    let b = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 20.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let (model, cert) = analytic_boolean(&a, &b, "intersection").unwrap();
    assert!(cert.all_complete());
    assert!(model.faces.is_empty() || model.validate().is_ok());
}

#[test]
fn multi_body_union_preserves_solidset() {
    let a = cylinder(2., 4.).unwrap();
    let b0 = cylinder(2., 4.).unwrap();
    let b = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 12.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let (model, cert) = analytic_boolean(&a, &b, "union").unwrap();
    assert!(cert.all_complete());
    assert!(!model.bodies.is_empty());
    model.validate().unwrap();
}

#[test]
fn imprint_reports_use_cap_planes_not_only_midplane() {
    let a = cylinder(2., 4.).unwrap();
    let b = cylinder(1., 4.).unwrap();
    let reports = imprint_reports(&a, &b).unwrap();
    assert!(reports.len() >= 4);
    assert!(reports.iter().all(|r| r.coverage == Coverage::Complete));
}

#[test]
fn intersecting_cylinders_union_without_prism() {
    let a = cylinder(2., 4.).unwrap();
    let b0 = cylinder(2., 4.).unwrap();
    let b = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 3.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let (model, cert) = analytic_boolean(&a, &b, "union").unwrap();
    assert!(cert.imprint_authored);
    assert!(cert.no_prism_authorship);
    assert!(cert.all_complete());
    model.validate().unwrap();
    let overlap_area = 8. * 0.75_f64.acos() - 1.5 * 7_f64.sqrt();
    let expected = (8. * std::f64::consts::PI - overlap_area) * 4.;
    let mass = crate::analysis::mass_properties(&model, 1e-7, 500_000).unwrap();
    assert!((mass.signed_volume_mm3 - expected).abs() < 1e-5);
}

#[test]
fn intersecting_cylinders_difference_without_prism() {
    let a = cylinder(2., 4.).unwrap();
    let b0 = cylinder(2., 4.).unwrap();
    let b = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 3.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let (model, cert) = analytic_boolean(&a, &b, "difference").unwrap();
    assert!(cert.no_prism_authorship);
    assert!(cert.all_complete());
    model.validate().unwrap();
    let overlap_area = 8. * 0.75_f64.acos() - 1.5 * 7_f64.sqrt();
    let expected = (4. * std::f64::consts::PI - overlap_area) * 4.;
    let mass = crate::analysis::mass_properties(&model, 1e-7, 500_000).unwrap();
    assert!((mass.signed_volume_mm3 - expected).abs() < 1e-5);
}

#[test]
fn intersecting_cylinders_intersection_exact_volume() {
    let a = cylinder(2., 4.).unwrap();
    let b0 = cylinder(2., 4.).unwrap();
    let b = crate::transform::affine(
        &b0,
        [
            [1., 0., 0., 3.],
            [0., 1., 0., 0.],
            [0., 0., 1., 0.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let (model, cert) = analytic_boolean(&a, &b, "intersection").unwrap();
    assert!(cert.no_prism_authorship);
    assert!(cert.all_complete());
    let overlap_area = 8. * 0.75_f64.acos() - 1.5 * 7_f64.sqrt();
    let mass = crate::analysis::mass_properties(&model, 1e-7, 500_000).unwrap();
    assert!((mass.signed_volume_mm3 - overlap_area * 4.).abs() < 1e-5);
}

#[test]
fn parallel_wall_ss_publishes_complete_generator_lines() {
    let report = crate::analytic_ss::cylinder_cylinder(
        FiniteCylinder {
            origin: [0., 0., 0.],
            direction: [0., 0., 1.],
            radius: 2.,
            height: [0., 4.],
        },
        FiniteCylinder {
            origin: [3., 0., 0.],
            direction: [0., 0., 1.],
            radius: 2.,
            height: [0., 4.],
        },
        Options::default(),
    )
    .unwrap();
    assert_eq!(report.coverage, Coverage::Complete);
    assert_eq!(report.components.len(), 2);
    assert!(matches!(
        report.components[0],
        crate::analytic_ss::AnalyticSsComponent::Line { .. }
    ));
}
