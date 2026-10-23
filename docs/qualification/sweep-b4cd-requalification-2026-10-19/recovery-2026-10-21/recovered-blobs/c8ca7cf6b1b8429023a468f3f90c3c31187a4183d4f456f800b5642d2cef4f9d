use nurbs_core::{curve::Curve, curve_tessellation::tessellate};
#[test]
fn rational_quadratic_chords_cover_analytic_curve_continuously() {
    let c = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 7., 7., 7.],
        control_points: vec![vec![0., 0.], vec![2., 3.], vec![4., 0.]],
        weights: vec![1., 2., 1.],
        periodic: false,
    };
    let r = tessellate(&c, 0.05, 10000).unwrap();
    assert!(r.within_tolerance);
    assert!(r.error_upper <= 0.05);
    assert_eq!(r.segments.first().unwrap().domain[0], 2.);
    assert_eq!(r.segments.last().unwrap().domain[1], 7.);
    for pair in r.segments.windows(2) {
        assert_eq!(pair[0].domain[1], pair[1].domain[0]);
        assert_eq!(pair[0].points[1], pair[1].points[0]);
    }
    for s in &r.segments {
        for i in 0..=10 {
            let t = i as f64 / 10.;
            let u = s.domain[0] + (s.domain[1] - s.domain[0]) * t;
            let q = (u - 2.) / 5.;
            let w = (1. - q).powi(2) + 4. * q * (1. - q) + q * q;
            let expected = [(8. * q * (1. - q) + 4. * q * q) / w, 12. * q * (1. - q) / w];
            let error = expected
                .iter()
                .enumerate()
                .map(|(k, x)| (x - (s.points[0][k] * (1. - t) + s.points[1][k] * t)).powi(2))
                .sum::<f64>()
                .sqrt();
            assert!(error <= s.error_upper);
        }
    }
}
#[test]
fn budget_stop_retains_the_entire_active_domain() {
    let c =
        Curve::from_polyline(vec![vec![0., 0., 0.], vec![1., 1., 0.], vec![2., 0., 0.]]).unwrap();
    assert!(tessellate(&c, 0.01, 1).is_err());
    let r = tessellate(&c, 0.01, 2).unwrap();
    assert!(!r.within_tolerance);
    assert_eq!(r.segments.len(), 2);
    assert_eq!(r.segments.first().unwrap().domain[0], c.domain()[0]);
    assert_eq!(r.segments.last().unwrap().domain[1], c.domain()[1]);
}
#[test]
fn invalid_tolerance_budget_and_source_fail() {
    let c = Curve::from_polyline(vec![vec![0., 0.], vec![1., 0.]]).unwrap();
    for t in [0., -1., f64::NAN] {
        assert!(tessellate(&c, t, 100).is_err());
    }
    for b in [0, 100001] {
        assert!(tessellate(&c, 0.1, b).is_err());
    }
    let mut invalid = c.clone();
    invalid.weights[0] = 0.;
    assert!(tessellate(&invalid, 0.1, 100).is_err());
}

#[test]
fn closed_circle_chords_stay_in_analytic_radial_band() {
    let c = nurbs_core::primitives::circle([0.; 3], [0., 0., 1.], 2.).unwrap();
    let r = tessellate(&c, 0.1, 10000).unwrap();
    assert!(r.within_tolerance);
    assert_eq!(
        r.segments.first().unwrap().points[0],
        r.segments.last().unwrap().points[1]
    );
    for s in &r.segments {
        for i in 0..=10 {
            let t = i as f64 / 10.;
            let p: Vec<_> = s.points[0]
                .iter()
                .zip(&s.points[1])
                .map(|(a, b)| a * (1. - t) + b * t)
                .collect();
            let radius = p.iter().map(|x| x * x).sum::<f64>().sqrt();
            assert!((radius - 2.).abs() <= s.error_upper);
        }
    }
}
