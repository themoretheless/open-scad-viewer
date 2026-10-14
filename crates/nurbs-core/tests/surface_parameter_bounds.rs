use nurbs_core::{surface::Surface, surface_parameter_bounds};
#[test]
fn affine_chart_derivative_bounds_cover_known_nonunit_axes() {
    let s = Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-1., -1., 3., 3.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 12., 16.]],
            vec![vec![15., 0., 0.], vec![15., 12., 16.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    };
    let b = surface_parameter_bounds::inspect(&s, 1).unwrap();
    assert_eq!(b.rectangles, 1);
    assert!(
        b.derivative_norm_upper_bounds[0] >= 3. && b.derivative_norm_upper_bounds[0].is_finite()
    );
    assert!(
        b.derivative_norm_upper_bounds[1] >= 5. && b.derivative_norm_upper_bounds[1].is_finite()
    );
    assert!(surface_parameter_bounds::inspect(&s, 0).is_err());
}

#[test]
fn rational_quarter_cylinder_bounds_cover_independent_quotient_derivatives() {
    let w = 0.5_f64.sqrt();
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![2., 0., 0.], vec![2., 0., 4.]],
            vec![vec![2., 2., 0.], vec![2., 2., 4.]],
            vec![vec![0., 2., 0.], vec![0., 2., 4.]],
        ],
        weights: vec![vec![1.; 2], vec![w; 2], vec![1.; 2]],
        periodic_u: false,
        periodic_v: false,
    };
    let b = surface_parameter_bounds::inspect(&s, 1).unwrap();
    assert!(b.derivative_norm_upper_bounds[1] >= 4.);
    for i in 0..=100 {
        let t = i as f64 / 100.;
        let denominator = (1. - t).powi(2) + 2. * w * t * (1. - t) + t * t;
        let d = -2. * (1. - t) + 2. * w * (1. - 2. * t) + 2. * t;
        let nx = 2. * (1. - t).powi(2) + 4. * w * t * (1. - t);
        let ny = 4. * w * t * (1. - t) + 2. * t * t;
        let dx =
            (-4. * (1. - t) + 4. * w * (1. - 2. * t)) / denominator - nx * d / denominator.powi(2);
        let dy = (4. * w * (1. - 2. * t) + 4. * t) / denominator - ny * d / denominator.powi(2);
        assert!(b.derivative_norm_upper_bounds[0] >= dx.hypot(dy));
    }
}
