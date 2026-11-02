use nurbs_core::{surface::Surface, surface_quotient_injectivity::certify_ruled_source_frame};
fn source() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![2., 2., 5., 5.],
        knots_v: vec![-4., -4., -4., 8., 8., 8.],
        control_points: vec![
            vec![vec![3., -7., 5.]; 3],
            vec![vec![4., -7., 5.], vec![2., -6., 5.], vec![1., -6., 5.]],
        ],
        weights: vec![vec![0.5, 2., 1.], vec![1., 0.5, 2.]],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn exact_dual_projection_admits_strong_shear_at_either_pole_end() {
    for end in [0, 1] {
        let mut s = source();
        if end == 1 {
            s.control_points.reverse();
            s.weights.reverse();
        }
        let saved = s.clone();
        let r = certify_ruled_source_frame(&s, end, 8, 16).unwrap();
        assert!(r.proven, "{r:?}");
        assert_eq!(r.reason, "global-ruled-dual-ray-quotient");
        assert_eq!(r.cells, 16);
        assert!(r.direction_denominator_lower.unwrap() > 0.);
        assert!(r.angular_derivative_numerator.unwrap()[0] > 0.);
        assert_eq!(s, saved);
    }
}
#[test]
fn dual_fallback_charges_both_attempts_and_cannot_hide_repeated_or_folded_rays() {
    let s = source();
    let r = certify_ruled_source_frame(&s, 0, 8, 15).unwrap();
    assert!(!r.proven && r.cells == 8 && r.reason == "work-limit");
    let mut repeated = s.clone();
    repeated.control_points[1][2] = repeated.control_points[1][0].clone();
    assert!(
        !certify_ruled_source_frame(&repeated, 0, 8, 16)
            .unwrap()
            .proven
    );
    let mut folded = s.clone();
    folded.control_points[1][1] = vec![6., -8., 5.];
    assert!(
        !certify_ruled_source_frame(&folded, 0, 8, 16)
            .unwrap()
            .proven
    );
    let mut bad = s.clone();
    bad.control_points[0][1][0] += 1e-12;
    assert!(!certify_ruled_source_frame(&bad, 0, 8, 16).unwrap().proven);
    assert_eq!(s, source());
}
#[test]
fn three_ray_frame_admits_each_strongly_sheared_quadrant_with_one_shared_budget() {
    let mut three_used = false;
    for points in [
        [[1., 0.], [1., 1.], [0., 1.]],
        [[0., 1.], [-1., 1.], [-1., 0.]],
        [[-1., 0.], [-1., -1.], [0., -1.]],
        [[0., -1.], [1., -1.], [1., 0.]],
    ] {
        let mut s = source();
        s.control_points[0] = vec![vec![3., -7., 7.]; 3];
        s.control_points[1] = points
            .iter()
            .map(|p| vec![p[0] - 16. * p[1] + 3., p[1] - 7., 5.])
            .collect();
        s.weights = vec![vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.]; 2];
        let r = certify_ruled_source_frame(&s, 0, 8, 24).unwrap();
        assert!(r.proven, "{r:?}");
        if r.reason == "global-ruled-three-ray-quotient" {
            three_used = true;
            assert_eq!(r.cells, 24);
            let small = certify_ruled_source_frame(&s, 0, 8, 23).unwrap();
            assert!(!small.proven && small.cells == 16 && small.reason == "work-limit");
        }
    }
    assert!(three_used);
}
#[test]
fn three_independent_frame_rays_do_not_admit_a_hidden_cubic_direction_fold() {
    // Original planar ray section has a crossing at two distinct interior
    // parameters. Three independent frame controls alone cannot admit it.
    let mut s = source();
    s.degree_v = 3;
    s.knots_v = vec![-4., -4., -4., -4., 8., 8., 8., 8.];
    s.control_points = vec![
        vec![vec![0., 0., 0.]; 4],
        vec![
            vec![0., 0., 1.],
            vec![2., 2., 1.],
            vec![-1., 2., 1.],
            vec![1., 0., 1.],
        ],
    ];
    s.weights = vec![vec![1.; 4]; 2];
    let saved = s.clone();
    let r = certify_ruled_source_frame(&s, 0, 16, 48).unwrap();
    assert!(!r.proven, "{r:?}");
    assert_eq!(r.cells, 48);
    assert_eq!(r.reason, "ray-direction-order-unproven");
    assert_eq!(s, saved);
}
