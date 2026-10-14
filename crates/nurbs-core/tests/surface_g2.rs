use nurbs_core::{
    polynomial,
    surface::{Axis, Surface},
    surface_edit,
    surface_g2::{self, Orientation},
    surface_join::{Boundary, Decision},
};
fn patch(left: bool, curvature: f64, nonlinear: bool) -> Surface {
    let mut c = vec![vec![[0.; 3]; 2]; if nonlinear { 5 } else { 3 }];
    c[1][0][0] = if nonlinear { 2. } else { 1. };
    c[0][1][1] = 1.;
    if nonlinear {
        c[2][0][0] = 3.;
        c[1][0][1] = 1.;
        c[2][0][2] = 4. * curvature;
        c[3][0][2] = 12. * curvature;
        c[4][0][2] = 9. * curvature;
    } else {
        c[2][0][2] = curvature;
    }
    polynomial::parametric_surface(
        if left {
            [-1., 0., 0., 1.]
        } else {
            [0., 1., 0., 1.]
        },
        &c,
    )
    .unwrap()
}
#[test]
fn geometric_g2_allows_speed_shear_and_nonlinear_parameter_freedom() {
    let a = patch(true, 1., false);
    let b = patch(false, 1., true);
    let before = b.clone();
    let r = surface_g2::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.05,
        0.02,
        0.1,
        1000,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance, "{r:?}");
    assert_eq!(r.orientation, Some(Orientation::Same));
    assert!(r.regularity_proven);
    assert!(r.branches[0].tensor_upper.unwrap() <= 0.1);
    assert_eq!(b, before);
}
#[test]
fn geometric_g2_accepts_different_boundary_degrees_and_knot_layouts() {
    let a = patch(true, 1., false);
    let elevated = surface_edit::elevate(&patch(false, 1., true), Axis::V, 3).unwrap();
    let b = surface_edit::insert(&elevated, Axis::V, 0.37, 1).unwrap();
    let before = (a.clone(), b.clone());
    let r = surface_g2::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.05,
        0.02,
        0.1,
        5000,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance, "{r:?}");
    assert_eq!(r.orientation, Some(Orientation::Same));
    assert!(r.regularity_proven);
    assert!(r.branches[0].tensor_upper.unwrap() <= 0.1);
    assert_eq!((a, b), before);
}
#[test]
fn opposite_normals_require_the_same_tensor_sign_branch() {
    let a = patch(true, 1., false);
    let b = surface_edit::reverse(&patch(false, 1., true), Axis::V).unwrap();
    let r = surface_g2::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        true,
        0.05,
        0.02,
        0.1,
        1000,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance, "{r:?}");
    assert_eq!(r.orientation, Some(Orientation::Opposite));
    assert!(r.branches[0].normal_lower > 0.02);
    assert!(r.branches[1].tensor_upper.unwrap() <= 0.1);
}
#[test]
fn g1_matching_planes_do_not_hide_a_curvature_gap() {
    let a = patch(true, 1., false);
    let b = patch(false, 2., false);
    let r = surface_g2::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.05,
        0.02,
        0.1,
        1000,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::CorrespondenceExceedsTolerance);
    assert!(r.branches[0].tensor_lower > 1.9 && r.branches[0].tensor_lower <= 2.);
    assert!(r.branches[1].normal_lower > 0.02);
    assert!(r.orientation.is_none());
}
#[test]
fn unavailable_smoothness_regularity_and_budget_never_report_g2() {
    let a = patch(true, 1., false);
    let b = surface_edit::insert(&patch(false, 1., false), Axis::V, 0.5, 1).unwrap();
    let r = surface_g2::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.05,
        0.02,
        0.1,
        31,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::Unresolved);
    assert!(!r.regularity_proven);
    assert!(r.branches[0].tensor_upper.is_none());
    let b = patch(false, 1., false);
    let r = surface_g2::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        1e-12,
        0.02,
        0.1,
        1,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::Unresolved);
    assert!(
        surface_g2::inspect(
            &a,
            Boundary::UMax,
            &b,
            Boundary::UMin,
            false,
            0.1,
            0.02,
            -1.,
            100
        )
        .is_err()
    );
    assert!(
        surface_g2::inspect(
            &a,
            Boundary::UMax,
            &b,
            Boundary::UMin,
            false,
            0.1,
            0.02,
            0.1,
            0
        )
        .is_err()
    );
    let mut a = a;
    let mut b = b;
    for s in [&mut a, &mut b] {
        for row in &mut s.control_points {
            for p in row {
                p[1] = 0.;
            }
        }
    }
    let r = surface_g2::inspect(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        0.05,
        0.02,
        0.1,
        11,
    )
    .unwrap();
    assert_eq!(r.decision, Decision::Unresolved);
    assert!(!r.regularity_proven);
}
