use nurbs_core::{periodic_trim::wrapped_surface_parameters as wrap, surface::Surface};
fn surface() -> Surface {
    let p = [[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]];
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![-1.5, -1., -0.5, 0., 0.5, 1., 1.5],
        knots_v: vec![-2., -2., 3., 3.],
        control_points: p
            .iter()
            .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 5.]])
            .collect(),
        weights: vec![vec![1., 1.]; 5],
        periodic_u: true,
        periodic_v: false,
    }
}
#[test]
fn explicit_sampling_wraps_signed_sheets_without_changing_strict_evaluation() {
    let s = surface();
    s.validate().unwrap();
    for shift in [-100., -4., 0., 2., 100.] {
        let uv = [0.25 + shift, 0.5];
        let mapped = wrap(&s, uv).unwrap();
        assert_eq!(mapped, [0.25, 0.5]);
        assert_eq!(
            s.evaluate(mapped[0], mapped[1]).unwrap().point,
            s.evaluate(0.25, 0.5).unwrap().point
        );
    }
    assert!(s.evaluate(2.25, 0.5).is_err());
    assert!(wrap(&s, [0., 3.01]).is_err());
    assert!(wrap(&s, [f64::NAN, 0.]).is_err());
    assert_eq!(wrap(&s, [1., 0.]).unwrap(), [-1., 0.]);
    assert_eq!(
        wrap(&s, [1f64.next_down(), 0.]).unwrap(),
        [1f64.next_down(), 0.]
    );
}
#[test]
fn both_periodic_axes_normalize_independently() {
    let mut s = surface();
    s.periodic_v = true;
    s.knots_v = vec![-1., 0., 1., 2., 3., 4., 5.];
    let v = [
        [0., 0., 0.],
        [0., 0., 1.],
        [0., 1., 1.],
        [0., 1., 0.],
        [0., 0., 0.],
    ];
    for row in &mut s.control_points {
        let p = row[0].clone();
        *row = v
            .iter()
            .map(|v| vec![p[0] + v[0], p[1] + v[1], p[2] + v[2]])
            .collect();
    }
    s.weights = vec![vec![1.; 5]; 5];
    s.validate().unwrap();
    assert_eq!(wrap(&s, [4.25, -7.5]).unwrap(), [0.25, 0.5]);
    assert_eq!(wrap(&s, [-3.75, 8.5]).unwrap(), [0.25, 0.5]);
}
