use nurbs_core::{curve::Curve, surface::Surface, weight_edit};
#[test]
fn curve_weights_follow_independent_rational_equation_and_preserve_basis() {
    let c =
        nurbs_core::paths::bezier(vec![vec![0., 0.], vec![2., 3.], vec![4., 0.]], None).unwrap();
    let t = weight_edit::curve(&c, &[(1, 4.)]).unwrap();
    assert_eq!(c.weights, vec![1.; 3]);
    assert_eq!(t.control_points, c.control_points);
    assert_eq!(t.knots, c.knots);
    for i in 0..=100 {
        let u = i as f64 / 100.;
        let a = (1. - u).powi(2);
        let b = 8. * u * (1. - u);
        let d = u * u;
        let w = a + b + d;
        let p = t.evaluate(u).unwrap().point;
        assert!((p[0] - (2. * b + 4. * d) / w).abs() < 1e-12);
        assert!((p[1] - 3. * b / w).abs() < 1e-12);
    }
    let scaled = weight_edit::curve(&t, &[(0, 2.), (1, 8.), (2, 2.)]).unwrap();
    for i in 0..=100 {
        let u = i as f64 / 100.;
        assert_eq!(
            t.evaluate(u).unwrap().point,
            scaled.evaluate(u).unwrap().point
        );
    }
}
fn periodic() -> Curve {
    Curve {
        degree: 2,
        knots: (0..11).map(|i| i as f64).collect(),
        control_points: vec![
            vec![1., 0., 0.],
            vec![0.5, 1., 0.],
            vec![-0.5, 1., 0.],
            vec![-1., 0., 0.],
            vec![-0.5, -1., 0.],
            vec![0.5, -1., 0.],
            vec![1., 0., 0.],
            vec![0.5, 1., 0.],
        ],
        weights: vec![1.; 8],
        periodic: true,
    }
}
#[test]
fn surface_local_weight_changes_match_bilinear_rational_formula() {
    let c = nurbs_core::primitives::line([0.; 3], [3., 0., 0.]).unwrap();
    let s = nurbs_core::surface::extrude(&c, [0., 2., 0.]).unwrap();
    let t = weight_edit::surface(&s, &[(1, 1, 4.)]).unwrap();
    assert_eq!(t.control_points, s.control_points);
    assert_eq!(t.knots_u, s.knots_u);
    assert_eq!(t.knots_v, s.knots_v);
    for i in 0..=40 {
        for j in 0..=40 {
            let u = i as f64 / 40.;
            let v = j as f64 / 40.;
            let w = 1. + 3. * u * v;
            let p = t.evaluate(u, v).unwrap().point;
            assert!((p[0] - 3. * u * (1. + 3. * v) / w).abs() < 1e-12);
            assert!((p[1] - 2. * v * (1. + 3. * u) / w).abs() < 1e-12);
        }
    }
}
#[test]
fn periodic_aliases_and_tensor_corner_copies_are_updated() {
    let c = periodic();
    let t = weight_edit::curve(&c, &[(7, 3.)]).unwrap();
    assert_eq!(t.weights[1], 3.);
    assert_eq!(t.weights[7], 3.);
    t.validate().unwrap();
    assert!(weight_edit::curve(&c, &[(1, 3.), (7, 3.)]).is_err());
    let points = c
        .control_points
        .iter()
        .map(|a| {
            c.control_points
                .iter()
                .map(|b| vec![a[0], b[0], a[1] + b[1]])
                .collect()
        })
        .collect();
    let s = Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: c.knots.clone(),
        knots_v: c.knots,
        control_points: points,
        weights: vec![vec![1.; 8]; 8],
        periodic_u: true,
        periodic_v: true,
    };
    let t = weight_edit::surface(&s, &[(7, 6, 4.)]).unwrap();
    for u in [1, 7] {
        for v in [0, 6] {
            assert_eq!(t.weights[u][v], 4.);
        }
    }
    t.validate().unwrap();
    assert!(weight_edit::surface(&s, &[(1, 0, 4.), (7, 6, 4.)]).is_err());
}
#[test]
fn invalid_weights_indices_and_conditioning_are_atomic() {
    let c = periodic();
    let before = c.clone();
    for value in [0., -1., f64::NAN, f64::INFINITY, 1e13, 1e-13] {
        assert!(weight_edit::curve(&c, &[(2, value)]).is_err());
    }
    assert!(weight_edit::curve(&c, &[]).is_err());
    assert!(weight_edit::curve(&c, &[(8, 2.)]).is_err());
    assert!(weight_edit::curve(&c, &[(2, 1e12), (3, 1e-12)]).is_err());
    assert_eq!(c, before);
    let s = nurbs_core::surface::extrude(&c, [0., 0., 2.]).unwrap();
    let before = s.clone();
    for value in [0., f64::NAN, 1e13] {
        assert!(weight_edit::surface(&s, &[(2, 1, value)]).is_err());
    }
    assert!(weight_edit::surface(&s, &[]).is_err());
    assert!(weight_edit::surface(&s, &[(0, 2, 1.)]).is_err());
    assert!(weight_edit::surface(&s, &[(2, 0, 1e12), (3, 1, 1e-12)]).is_err());
    assert_eq!(s, before);
}
