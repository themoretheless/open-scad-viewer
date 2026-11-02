use nurbs_core::{
    surface::{Axis, Surface},
    surface_offset::{CandidateStatus as S, certify_mapped_candidate},
};
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn changed_basis_and_source_subrectangle_are_verified_over_the_whole_cap() {
    let s = plane();
    let mut c =
        nurbs_core::surface_edit::elevate(&s.trim([3., 6., -2., 0.]).unwrap(), Axis::U, 2).unwrap();
    for p in c.control_points.iter_mut().flatten() {
        p[2] += 0.1;
    }
    let r = certify_mapped_candidate(&s, &c, [[3., 6.], [-2., 0.]], 0.1, 1e-7, 4096).unwrap();
    assert_eq!(r.status, S::Within);
    assert!(r.error_upper.unwrap() <= 1e-7);
    assert_eq!(
        certify_mapped_candidate(&s, &c, [[2., 7.], [-3., 1.]], 0.1, 0.01, 4096)
            .unwrap()
            .status,
        S::Exceeds
    );
}
#[test]
fn invalid_windows_and_budget_stops_never_provide_a_certificate() {
    let s = plane();
    for w in [
        [[1., 7.], [-3., 1.]],
        [[7., 2.], [-3., 1.]],
        [[2., 2.], [-3., 1.]],
        [[2., 7.], [f64::NAN, 1.]],
    ] {
        assert!(certify_mapped_candidate(&s, &s, w, 0.1, 0.01, 10).is_err());
    }
    let r = certify_mapped_candidate(&s, &s, [[2., 7.], [-3., 1.]], 0.1, 0.01, 1).unwrap();
    assert_ne!(r.status, S::Within);
    assert!(r.error_upper.is_none());
}

#[test]
fn independent_domain_reparameterization_and_interior_drift_are_checked() {
    let s = plane();
    let mut c = s.trim([3., 6., -2., 0.]).unwrap();
    c.knots_u = vec![10., 10., 20., 20.];
    c.knots_v = vec![4., 4., 8., 8.];
    for p in c.control_points.iter_mut().flatten() {
        p[2] = 0.1;
    }
    let w = [[3., 6.], [-2., 0.]];
    assert_eq!(
        certify_mapped_candidate(&s, &c, w, 0.1, 1e-7, 4096)
            .unwrap()
            .status,
        S::Within
    );
    let mut c = nurbs_core::surface_edit::elevate(&c, Axis::U, 2).unwrap();
    for p in &mut c.control_points[1] {
        p[2] += 0.2;
    }
    assert_eq!(
        certify_mapped_candidate(&s, &c, w, 0.1, 0.01, 4096)
            .unwrap()
            .status,
        S::Exceeds
    );
}

#[test]
fn no_sample_can_admit_an_unresolved_curved_chart() {
    let mut s = plane();
    s.control_points[1][1][2] = 0.5;
    let mut c = s.clone();
    for p in c.control_points.iter_mut().flatten() {
        p[2] += 0.1;
    }
    let r = certify_mapped_candidate(&s, &c, [[2., 7.], [-3., 1.]], 0.1, 1e-12, 1).unwrap();
    assert_eq!(r.status, S::Unresolved);
    assert!(r.error_upper.is_none());
    let mut s = plane();
    for p in s.control_points.iter_mut().flatten() {
        p[1] = 0.;
    }
    assert_eq!(
        certify_mapped_candidate(&s, &s, [[2., 7.], [-3., 1.]], 0.1, 0.01, 8)
            .unwrap()
            .status,
        S::Unresolved
    );
}
