use nurbs_core::{
    curve::Curve,
    curve_pullback::{self, Limits},
    surface::{Axis, Surface},
};
fn limits() -> Limits {
    Limits {
        tolerance: 1e-7,
        max_segments: 32,
        max_iterations: 32,
        agreement_cells: 100000,
    }
}
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 4., 4.],
        knots_v: vec![-1., -1., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 2., 0.]],
            vec![vec![2., 0., 0.], vec![2., 2., 0.]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn affine_pullback_preserves_rational_definition_and_known_uv() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![vec![0.25, 0.5, 0.], vec![1., 1., 0.], vec![1.75, 1.5, 0.]],
        weights: vec![1., 2., 1.],
        periodic: false,
    };
    let s = plane();
    let before = (c.clone(), s.clone());
    let r = curve_pullback::project(&c, &s, limits()).unwrap();
    assert!(r.within_tolerance);
    let p = r.pcurve.unwrap();
    assert_eq!(p.degree, c.degree);
    assert_eq!(p.knots, c.knots);
    assert_eq!(p.weights, c.weights);
    assert_eq!(p.control_points[0], vec![2.25, -0.5]);
    assert_eq!((c, s), before);
}
#[test]
fn general_curved_support_uses_full_interval_admission() {
    let s = Surface {
        degree_u: 2,
        degree_v: 1,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![vec![0.5, 0., 0.5], vec![0.5, 1., 0.5]],
            vec![vec![1., 0., 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 2]; 3],
        periodic_u: false,
        periodic_v: false,
    };
    let c = s.iso(Axis::V, 0.5).unwrap();
    let r = curve_pullback::project(&c, &s, limits()).unwrap();
    assert!(r.within_tolerance, "{r:?}");
    assert!(r.evaluations > 0);
    let p = r.pcurve.unwrap();
    for t in [0., 0.25, 0.5, 0.75, 1.] {
        let uv = p.evaluate(t).unwrap().point;
        assert!((uv[0] - t).abs() < 1e-6 && (uv[1] - 0.5).abs() < 1e-6);
    }
}
#[test]
fn off_surface_and_exhausted_requests_are_not_certified() {
    let c = Curve::from_polyline(vec![vec![0., 0., 0.1], vec![1., 1., 0.1]]).unwrap();
    let r = curve_pullback::project(
        &c,
        &plane(),
        Limits {
            max_segments: 1,
            ..limits()
        },
    )
    .unwrap();
    assert!(!r.within_tolerance);
    assert!(r.pcurve.is_none());
    assert!(
        curve_pullback::project(
            &c,
            &plane(),
            Limits {
                agreement_cells: 0,
                ..limits()
            }
        )
        .is_err()
    );
    assert!(
        curve_pullback::project(
            &c,
            &plane(),
            Limits {
                max_iterations: 0,
                ..limits()
            }
        )
        .is_err()
    );
}
