use super::*;
#[test]
fn discontinuity_cannot_receive_a_lipschitz_certificate() {
    let curve = Curve {
        degree: 1,
        knots: vec![0., 0., 0.5, 0.5, 1., 1.],
        control_points: vec![
            vec![0., 0., 0.],
            vec![1., 0., 0.],
            vec![2., 0., 0.],
            vec![3., 0., 0.],
        ],
        weights: vec![1.; 4],
        periodic: false,
    };
    assert!(periodic_error(&curve, &curve).is_err());
}
#[test]
fn narrow_peak_between_samples_is_bounded() {
    for scale in [0.1, 1., 10.] {
        let source = Curve {
            degree: 1,
            knots: vec![0., 0., 0.0001 * scale, 0.0002 * scale, scale, scale],
            control_points: vec![
                vec![0., 0., 0.],
                vec![0., 1., 0.],
                vec![0., 0., 0.],
                vec![0., 0., 0.],
            ],
            weights: vec![1.; 4],
            periodic: false,
        };
        let target = Curve {
            degree: 1,
            knots: vec![0., 0., scale, scale],
            control_points: vec![vec![0., 0., 0.]; 2],
            weights: vec![1.; 2],
            periodic: false,
        };
        let error = periodic_error(&source, &target).unwrap();
        assert!(error >= 1., "missed narrow peak: {error}");
        assert!((error - 4.8828125).abs() < 1e-10);
    }
}
