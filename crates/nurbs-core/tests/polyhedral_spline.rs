use nurbs_core::polyhedral_spline::{self, Boundary, Continuity, PolyhedralSpline};
fn cage(offset: [f64; 3]) -> PolyhedralSpline {
    PolyhedralSpline::from_regular_grid(
        (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| {
                        [
                            offset[0] + i as f64 / 3.,
                            offset[1] + j as f64 / 3.,
                            offset[2],
                        ]
                    })
                    .collect()
            })
            .collect(),
    )
    .unwrap()
}
#[test]
fn retained_quad_cage_evaluates_as_geometry_and_edits_locally() {
    let a = cage([0.; 3]);
    let p = a.surface().evaluate(0.3, 0.7).unwrap().point;
    assert!((p[0] - 0.3).abs() < 1e-12);
    assert!((p[1] - 0.7).abs() < 1e-12);
    let b = a.with_control_point(1, 1, [1. / 3., 1. / 3., 1.]).unwrap();
    assert_eq!(a.control_point(1, 1).unwrap()[2], 0.);
    assert!(b.surface().evaluate(0.4, 0.4).unwrap().point[2] > 0.);
    assert_eq!(b.patches().unwrap().len(), 1);
}
#[test]
fn reconciliation_matches_complete_seam_and_derivative_field() {
    let a = cage([0.; 3]);
    let b = cage([1.2, 0., 0.2]);
    let result = polyhedral_spline::reconcile(
        &a,
        Boundary::UMax,
        &b,
        Boundary::UMin,
        false,
        Continuity::ParametricC1,
    )
    .unwrap();
    for i in 0..=20 {
        let t = i as f64 / 20.;
        let x = result.first.surface().evaluate(1., t).unwrap();
        let y = result.second.surface().evaluate(0., t).unwrap();
        for k in 0..3 {
            assert!((x.point[k] - y.point[k]).abs() < 1e-12);
        }
        let (dx, _) = x.first_derivatives().unwrap();
        let (dy, _) = y.first_derivatives().unwrap();
        for k in 0..3 {
            assert!((dx[k] - dy[k]).abs() < 1e-11);
        }
    }
    assert!(!result.regularity_certified);
    assert!(result.max_control_displacement > 0.);
}
#[test]
fn invalid_topology_and_control_updates_fail() {
    assert!(PolyhedralSpline::from_regular_grid(vec![vec![[0.; 3]; 3]; 3]).is_err());
    let a = cage([0.; 3]);
    assert!(a.with_control_point(7, 0, [0.; 3]).is_err());
    assert!(a.with_control_point(1, 1, [f64::NAN, 0., 0.]).is_err());
}
