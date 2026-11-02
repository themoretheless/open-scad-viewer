use nurbs_core::{
    surface::Surface,
    surface_offset::{CandidateStatus as S, certify_candidate},
};
fn patch(bend: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 1., bend]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn shifted(s: &Surface, d: f64) -> Surface {
    let mut c = s.clone();
    for p in c.control_points.iter_mut().flatten() {
        p[2] += d;
    }
    c
}
#[test]
fn continuous_plane_and_signed_offsets() {
    for d in [-0.2, 0., 0.2] {
        let s = patch(0.);
        let c = shifted(&s, d);
        let r = certify_candidate(&s, &c, d, 1e-10, 64).unwrap();
        assert_eq!(r.status, S::Within);
        assert!(r.error_upper.unwrap() <= 1e-10);
    }
}
#[test]
fn nonplanar_candidate_whole_domain() {
    let s = patch(0.2);
    let c = shifted(&s, 0.1);
    let before = s.clone();
    let r = certify_candidate(&s, &c, 0.1, 0.05, 4096).unwrap();
    assert_eq!(r.status, S::Within);
    assert_eq!(s, before);
    for i in 0..=20 {
        for j in 0..=20 {
            let u = 2. + 5. * i as f64 / 20.;
            let v = -3. + 4. * j as f64 / 20.;
            let a = s.evaluate(u, v).unwrap();
            let b = c.evaluate(u, v).unwrap();
            let n = a.unit_normal().unwrap();
            let error = (0..3)
                .map(|k| (b.point[k] - a.point[k] - 0.1 * n[k]).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(error <= r.error_upper.unwrap());
        }
    }
}
#[test]
fn wrong_side_and_lateral_drift_are_rejected() {
    let s = patch(0.);
    for mut c in [shifted(&s, -0.1), shifted(&s, 0.1)] {
        if c.control_points[0][0][2] > 0. {
            for p in c.control_points.iter_mut().flatten() {
                p[0] += 0.2;
            }
        }
        assert_eq!(
            certify_candidate(&s, &c, 0.1, 0.01, 64).unwrap().status,
            S::Exceeds
        );
    }
}
#[test]
fn exhaustion_degenerate_normals_and_changed_basis_fail_closed() {
    let s = patch(0.8);
    let c = shifted(&s, 0.1);
    let r = certify_candidate(&s, &c, 0.1, 1e-12, 1).unwrap();
    assert_eq!(r.status, S::Unresolved);
    assert!(r.error_upper.is_none());
    let mut c = s.clone();
    c.weights[0][0] = 2.;
    assert!(certify_candidate(&s, &c, 0.1, 0.1, 10).is_err());
    let mut flat = patch(0.);
    for p in flat.control_points.iter_mut().flatten() {
        p[1] = 0.;
    }
    assert_eq!(
        certify_candidate(&flat, &flat, 0.1, 0.1, 8).unwrap().status,
        S::Unresolved
    );
}

#[test]
fn interior_bulge_is_not_hidden_by_matching_corner_offsets() {
    let mut s = patch(0.);
    s.degree_u = 2;
    s.knots_u = vec![2., 2., 2., 7., 7., 7.];
    s.control_points
        .insert(1, vec![vec![0.5, 0., 0.], vec![0.5, 1., 0.]]);
    s.weights = vec![vec![1., 2.], vec![0.75, 1.5], vec![1., 2.]];
    let mut c = shifted(&s, 0.1);
    c.control_points[1][0][2] += 0.5;
    c.control_points[1][1][2] += 0.5;
    assert_eq!(
        certify_candidate(&s, &c, 0.1, 0.02, 4096).unwrap().status,
        S::Exceeds
    );
}

#[test]
fn invalid_controls_and_limits_are_errors() {
    let s = patch(0.);
    for t in [0., -1., f64::NAN, f64::INFINITY] {
        assert!(certify_candidate(&s, &s, 0.1, t, 8).is_err());
    }
    assert!(certify_candidate(&s, &s, f64::NAN, 0.1, 8).is_err());
    assert!(certify_candidate(&s, &s, 0.1, 0.1, 0).is_err());
    let mut c = s.clone();
    for p in c.control_points.iter_mut().flatten() {
        p.pop();
    }
    assert!(certify_candidate(&c, &c, 0.1, 0.1, 8).is_err());
}

#[test]
fn rational_cylinder_offset_has_continuous_assurance() {
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![1., 0., 0.], vec![1., 0., 2.]],
            vec![vec![1., 1., 0.], vec![1., 1., 2.]],
            vec![vec![0., 1., 0.], vec![0., 1., 2.]],
        ],
        weights: vec![
            vec![1.; 2],
            vec![std::f64::consts::FRAC_1_SQRT_2; 2],
            vec![1.; 2],
        ],
        periodic_u: false,
        periodic_v: false,
    };
    for d in [-0.1, 0.1] {
        let mut c = s.clone();
        for p in c.control_points.iter_mut().flatten() {
            p[0] *= 1. + d;
            p[1] *= 1. + d;
        }
        let r = certify_candidate(&s, &c, d, 0.01, 32768).unwrap();
        assert_eq!(r.status, S::Within);
        assert!(r.error_upper.unwrap() <= 0.01);
    }
}
