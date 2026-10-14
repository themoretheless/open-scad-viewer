use nurbs_core::{curve::Curve, curve_offset::approximate_curve};
fn arc() -> Curve {
    Curve {
        degree: 2,
        knots: vec![2., 2., 2., 7., 7., 7.],
        control_points: vec![vec![1., 0.], vec![1., 1.], vec![0., 1.]],
        weights: vec![1., std::f64::consts::FRAC_1_SQRT_2, 1.],
        periodic: false,
    }
}
#[test]
fn rational_circle_offset_follows_independent_radial_formula() {
    for d in [-0.25, 0.25, 1.25] {
        let r = approximate_curve(&arc(), d, 1e-3, 65536).unwrap();
        assert!(r.error_upper_mm <= 1e-3);
        assert_eq!(r.segments.first().unwrap().domain[0], 2.);
        assert_eq!(r.segments.last().unwrap().domain[1], 7.);
        for s in &r.segments {
            for i in 0..=10 {
                let q = i as f64 / 10.;
                let u = s.domain[0] * (1. - q) + s.domain[1] * q;
                let t = (u - 2.) / 5.;
                let a = (1. - t).powi(2);
                let b = std::f64::consts::SQRT_2 * t * (1. - t);
                let c = t * t;
                let w = a + b + c;
                let expected = [(1. - d) * (a + b) / w, (1. - d) * (b + c) / w];
                let chord = [
                    s.points[0][0] * (1. - q) + s.points[1][0] * q,
                    s.points[0][1] * (1. - q) + s.points[1][1] * q,
                ];
                assert!((chord[0] - expected[0]).hypot(chord[1] - expected[1]) <= s.error_upper_mm);
            }
        }
        for pair in r.curves.windows(2) {
            assert_eq!(pair[0].domain()[1], pair[1].domain()[0]);
            assert_eq!(
                pair[0].control_points.last(),
                pair[1].control_points.first()
            );
        }
    }
}
#[test]
fn zero_offset_is_identity_and_corners_require_a_join() {
    let a = arc();
    let z = approximate_curve(&a, 0., 1e-3, 1).unwrap();
    assert_eq!(z.curves, vec![a.clone()]);
    assert_eq!(z.error_upper_mm, 0.);
    let c = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.], vec![1., 1.]]).unwrap();
    assert!(approximate_curve(&c, 0.1, 1e-3, 100).is_err());
    assert!(approximate_curve(&a, 0.25, 1e-9, 1).is_err());
    assert_eq!(a, arc());
}
#[test]
fn singular_and_invalid_requests_fail() {
    let cusp = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![1., 0.], vec![-1., 0.], vec![1., 0.]],
        weights: vec![1.; 3],
        periodic: false,
    };
    assert!(approximate_curve(&cusp, 0.2, 1e-3, 100).is_err());
    let a = arc();
    for t in [0., -1., f64::NAN] {
        assert!(approximate_curve(&a, 0.25, t, 100).is_err());
    }
    for budget in [0, 65537] {
        assert!(approximate_curve(&a, 0.25, 1e-3, budget).is_err());
    }
    assert!(approximate_curve(&a, f64::INFINITY, 1e-3, 100).is_err());
}
