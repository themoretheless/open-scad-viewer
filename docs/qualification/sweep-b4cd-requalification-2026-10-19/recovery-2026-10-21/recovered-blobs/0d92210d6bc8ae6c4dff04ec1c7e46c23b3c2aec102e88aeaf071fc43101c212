use nurbs_core::{
    surface::{Axis, Surface},
    surface_edit,
    surface_join::{Boundary, Decision, inspect_g0},
};
fn panel(x: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: vec![
            vec![vec![x, 0., 0.], vec![x, 4., 0.]],
            vec![vec![x + 4., 0., 0.], vec![x + 4., 4., 0.]],
        ],
        weights: vec![vec![1., 2.], vec![3., 6.]],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn rational_boundaries_match_and_reverse_is_explicit() {
    let a = panel(0.);
    let b = panel(4.);
    let r = inspect_g0(&a, Boundary::UMax, &b, Boundary::UMin, false, 0.02, 4096).unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance);
    let reversed = surface_edit::reverse(&b, Axis::V).unwrap();
    assert_eq!(
        inspect_g0(
            &a,
            Boundary::UMax,
            &reversed,
            Boundary::UMin,
            true,
            0.02,
            4096
        )
        .unwrap()
        .decision,
        Decision::WithinTolerance
    );
    assert_eq!(
        inspect_g0(
            &a,
            Boundary::UMax,
            &reversed,
            Boundary::UMin,
            false,
            0.02,
            4096
        )
        .unwrap()
        .decision,
        Decision::CorrespondenceExceedsTolerance
    );
}
#[test]
fn gap_budget_and_invalid_requests_are_explicit() {
    let a = panel(0.);
    let b = panel(4.25);
    let r = inspect_g0(&a, Boundary::UMax, &b, Boundary::UMin, false, 0.01, 100).unwrap();
    assert_eq!(r.decision, Decision::CorrespondenceExceedsTolerance);
    assert!(r.bounds[0] <= 0.25 && 0.25 <= r.bounds[1]);
    let same = panel(4.);
    assert_eq!(
        inspect_g0(&a, Boundary::UMax, &same, Boundary::UMin, false, 0.01, 1)
            .unwrap()
            .decision,
        Decision::Unresolved
    );
    for t in [-1., f64::NAN] {
        assert!(inspect_g0(&a, Boundary::UMax, &same, Boundary::UMin, false, t, 100).is_err());
    }
    for budget in [0, 100001] {
        assert!(
            inspect_g0(
                &a,
                Boundary::UMax,
                &same,
                Boundary::UMin,
                false,
                0.01,
                budget
            )
            .is_err()
        );
    }
}

#[test]
fn unclamped_boundary_and_different_domains_use_actual_surface_image() {
    let mut a = panel(0.);
    a.degree_u = 2;
    a.knots_u = vec![0., 1., 2., 3., 4., 5.];
    a.control_points = vec![
        vec![vec![0., 0., 0.], vec![0., 4., 0.]],
        vec![vec![2., 0., 0.], vec![2., 4., 0.]],
        vec![vec![4., 0., 0.], vec![4., 4., 0.]],
    ];
    a.weights = vec![vec![1., 2.]; 3];
    let mut b = panel(3.);
    b.knots_v = vec![10., 10., 20., 20.];
    let r = inspect_g0(&a, Boundary::UMax, &b, Boundary::UMin, false, 0.02, 4096).unwrap();
    assert_eq!(r.decision, Decision::WithinTolerance, "{r:?}");
}
