use nurbs_core::{curve::Curve, offset::profile_corner::full_round::propose};
fn line(a: [f64; 2], b: [f64; 2]) -> Curve {
    Curve::from_polyline(vec![a.to_vec(), b.to_vec()]).unwrap()
}
#[test]
fn full_round_contacts_recover_parallel_support_semicircle() {
    let a = line([4., 0.], [4., 8.]);
    let m = line([4., 8.], [0., 8.]);
    let b = line([0., 8.], [0., 0.]);
    let p = propose(&a, &m, &b, 1e-8).unwrap();
    assert!((p.radius() - 2.).abs() < 1e-10);
    assert!(
        p.center()
            .iter()
            .zip([2., 6.])
            .all(|(a, b)| (a - b).abs() < 1e-10)
    );
    assert!(p.contact_residual() < 1e-8);
    assert_eq!(
        p.bridges()[0].control_points.last(),
        p.bridges()[1].control_points.first()
    );
    assert_eq!(
        p.incoming().control_points.last(),
        p.bridges()[0].control_points.first()
    );
    assert_eq!(
        p.outgoing().control_points.first(),
        p.bridges()[1].control_points.last()
    );
}
#[test]
fn full_round_solves_original_polynomial_and_rational_curved_supports() {
    for weight in [1., 0.7] {
        let curve = |points: [[f64; 2]; 3], domain: [f64; 2]| Curve {
            degree: 2,
            knots: vec![
                domain[0], domain[0], domain[0], domain[1], domain[1], domain[1],
            ],
            control_points: points.map(|p| p.to_vec()).to_vec(),
            weights: vec![1., weight, 1.],
            periodic: false,
        };
        let a = curve([[4., 0.], [5., 4.], [4., 8.]], [-3., 7.]);
        let b = curve([[0., 8.], [-1., 4.], [0., 0.]], [11., 31.]);
        let m = line([4., 8.], [0., 8.]);
        let before = [a.clone(), m.clone(), b.clone()];
        let p = propose(&a, &m, &b, 1e-8).unwrap();
        assert_eq!([a.clone(), m, b.clone()], before);
        assert!(p.radius() > 2.);
        assert!((p.center()[0] - 2.).abs() < 1e-8);
        assert!((p.center()[1] + p.radius() - 8.).abs() < 1e-8);
        assert!(p.source_error_upper().iter().all(|e| *e < 1e-8));
        assert!(p.contact_residual() < 1e-8);
        assert!((1..=40).contains(&p.iterations()));
        for (i, c) in [&a, &b].into_iter().enumerate() {
            let t = p.parameters()[i];
            let d = c.domain();
            assert!(t > d[0] && t < d[1]);
            let e = c.evaluate(t).unwrap();
            let v = e.d1.unwrap();
            let speed = v[0].hypot(v[1]);
            let expected = [
                e.point[0] - p.radius() * v[1] / speed,
                e.point[1] + p.radius() * v[0] / speed,
            ];
            assert!((expected[0] - p.center()[0]).hypot(expected[1] - p.center()[1]) < 1e-8);
        }
    }
}
#[test]
fn full_round_refuses_nonfitting_and_invalid_original_contacts() {
    let a = line([4., 7.], [4., 8.]);
    let m = line([4., 8.], [0., 8.]);
    let b = line([0., 8.], [0., 7.]);
    assert!(propose(&a, &m, &b, 1e-8).is_err());
    assert!(propose(&a, &m, &b, f64::NAN).is_err());
    assert!(propose(&a.reverse().unwrap(), &m, &b, 1e-8).is_err());
}
