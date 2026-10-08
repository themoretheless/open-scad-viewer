use nurbs_core::{bounds::Bounds, coordinate_frame as frame};
#[test]
fn weighted_2d_curve_normalizes_and_restores_independent_equation() {
    let c = nurbs_core::paths::bezier(
        vec![vec![10., 20.], vec![12., 24.], vec![14., 20.]],
        Some(vec![1., 2., 1.]),
    )
    .unwrap();
    let f = frame::Frame::from_bounds(&c.bounds().unwrap()).unwrap();
    assert_eq!(f.origin, vec![12., 22.]);
    assert_eq!(f.scale, 0.5);
    let q = frame::curve(&c, &f, false).unwrap();
    let r = frame::curve(&q, &f, true).unwrap();
    assert_eq!(r, c);
    assert_eq!(q.weights, c.weights);
    assert_eq!(q.knots, c.knots);
    for i in 0..=100 {
        let t = i as f64 / 100.;
        let w = 1. + 2. * t * (1. - t);
        let x = (10. * (1. - t).powi(2) + 48. * t * (1. - t) + 14. * t * t) / w;
        let y = (20. * (1. - t).powi(2) + 96. * t * (1. - t) + 20. * t * t) / w;
        let p = q.evaluate(t).unwrap().point;
        assert!((p[0] - (x - 12.) / 2.).abs() < 1e-12);
        assert!((p[1] - (y - 22.) / 2.).abs() < 1e-12);
    }
}
#[test]
fn surface_frame_and_unit_conversion_preserve_basis() {
    let c = nurbs_core::primitives::line([10., 20., 30.], [14., 20., 30.]).unwrap();
    let s = nurbs_core::surface::extrude(&c, [0., 2., 0.]).unwrap();
    let f = frame::Frame::from_bounds(&s.bounds().unwrap()).unwrap();
    let local = frame::surface(&s, &f, false).unwrap();
    assert_eq!(frame::surface(&local, &f, true).unwrap(), s);
    let mm = frame::scale_surface(&s, 1000.).unwrap();
    let m = frame::scale_surface(&mm, 0.001).unwrap();
    assert_eq!(m, s);
    for (u, v) in [(0., 0.), (0.25, 0.75), (1., 1.)] {
        let p = local.evaluate(u, v).unwrap().point;
        assert!((p[0] - (2. * u - 1.)).abs() < 1e-12);
        assert!((p[1] - (v - 0.5)).abs() < 1e-12);
    }
}
#[test]
fn point_boxes_tiny_geometry_and_invalid_frames() {
    let f = frame::Frame::from_bounds(&Bounds {
        min: vec![2., 3.],
        max: vec![2., 3.],
    })
    .unwrap();
    assert_eq!(f.scale, 1.);
    let c = nurbs_core::primitives::line([0.; 3], [1e-110, 0., 0.]).unwrap();
    let f = frame::Frame::from_bounds(&c.bounds().unwrap()).unwrap();
    let q = frame::curve(&c, &f, false).unwrap();
    assert!((q.control_points[0][0] + 1.).abs() < 1e-12);
    assert!((q.control_points[1][0] - 1.).abs() < 1e-12);
    for scale in [0., -1., f64::NAN, f64::INFINITY, 1e120] {
        assert!(frame::scale_curve(&c, scale).is_err());
    }
    assert!(frame::curve(
        &c,
        &frame::Frame {
            origin: vec![0.; 2],
            scale: 1.
        },
        false
    )
    .is_err());
    assert!(frame::Frame::from_bounds(&Bounds {
        min: vec![3., 0.],
        max: vec![2., 1.]
    })
    .is_err());
}
