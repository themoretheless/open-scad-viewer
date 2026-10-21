use nurbs_core::{
    curve::Curve,
    curve_surface_agreement::{self, Status},
    surface::Surface,
};
fn line(a: Vec<f64>, b: Vec<f64>) -> Curve {
    Curve::from_polyline(vec![a, b]).unwrap()
}
fn surface() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 2,
        knots_u: vec![2., 2., 5., 5.],
        knots_v: vec![-2., -2., -2., 4., 4., 4.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 0.125, 0.], vec![0., 1., 0.]],
            vec![vec![1., 0., 0.], vec![1., 0.125, 0.], vec![1., 1., 0.]],
        ],
        weights: vec![vec![1.; 3]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn nonlinear_original_correspondence_covers_all_segments_and_rounding() {
    let s = surface();
    let mut c = line(vec![0., 0., 0.], vec![0., 1., 0.]);
    c.knots = vec![7., 7., 9., 9.];
    let p = line(vec![2., -2.], vec![2., 4.]);
    assert_eq!(
        curve_surface_agreement::verify(&c, &p, &s, false, 1e-4, 1000)
            .unwrap()
            .status,
        Status::Mismatch
    );
    let nodes: Vec<_> = (0..=64)
        .map(|i| {
            let v = i as f64 / 64.;
            [0.25 * v + 0.75 * v * v, v]
        })
        .collect();
    let r =
        curve_surface_agreement::verify_correspondence(&c, &p, &s, &nodes, 1e-4, 10000).unwrap();
    assert_eq!(r.status, Status::WithinTolerance);
    assert!(r.unresolved_relation.is_empty());
    let bound = r.distance_upper.unwrap();
    assert!(bound <= 1e-4);
    for w in nodes.windows(2) {
        for i in 0..=10 {
            let t = i as f64 / 10.;
            let y = w[0][0] + (w[1][0] - w[0][0]) * t;
            let v = w[0][1] + (w[1][1] - w[0][1]) * t;
            assert!((y - (0.25 * v + 0.75 * v * v)).abs() <= bound + 2e-15);
        }
    }
    let reversed: Vec<_> = nodes.iter().rev().copied().collect();
    assert_eq!(
        curve_surface_agreement::verify_correspondence(&c, &p, &s, &reversed, 1e-4, 10000)
            .unwrap()
            .status,
        Status::WithinTolerance
    );
    let r = curve_surface_agreement::verify_correspondence(&c, &p, &s, &nodes, 1e-4, 1).unwrap();
    assert_eq!(r.status, Status::Unresolved);
    assert_eq!(r.unresolved_relation, nodes);
    assert!(r.distance_upper.is_none());
}
#[test]
fn rational_map_and_partial_reverse_preserve_original_domains() {
    let mut s = surface();
    s.degree_v = 1;
    s.knots_v = vec![0., 0., 1., 1.];
    s.control_points = vec![
        vec![vec![0., 0., 0.], vec![0., 1., 0.]],
        vec![vec![1., 0., 0.], vec![1., 1., 0.]],
    ];
    s.weights = vec![vec![1., 2.]; 2];
    let c = line(vec![0., 0., 0.], vec![0., 1., 0.]);
    let p = line(vec![2., 1.], vec![2., 0.]);
    let nodes: Vec<_> = (0..=128)
        .map(|i| {
            let v = 0.25 + 0.5 * i as f64 / 128.;
            [2. * v / (1. + v), 1. - v]
        })
        .collect();
    let r =
        curve_surface_agreement::verify_correspondence(&c, &p, &s, &nodes, 1e-4, 10000).unwrap();
    assert_eq!(r.status, Status::WithinTolerance);
    assert!(r.distance_upper.unwrap() <= 1e-4);
    // A partial-range success says nothing about the omitted end portions.
    assert!(nodes[0][0] > 0. && nodes.last().unwrap()[0] < 1.);
    let mut bad = s.clone();
    for point in bad.control_points.iter_mut().flatten() {
        point[2] += 0.01;
    }
    assert_eq!(
        curve_surface_agreement::verify_correspondence(&c, &p, &bad, &nodes, 1e-4, 10000)
            .unwrap()
            .status,
        Status::Mismatch
    );
}
#[test]
fn malformed_maps_and_limits_are_refused() {
    let s = surface();
    let c = line(vec![0., 0., 0.], vec![0., 1., 0.]);
    let p = line(vec![2., -2.], vec![2., 4.]);
    for nodes in [
        vec![],
        vec![[0., 0.]],
        vec![[0., 0.], [0., 1.]],
        vec![[0., 0.], [0.8, 0.8], [0.7, 1.]],
        vec![[0., 0.], [1., f64::NAN]],
    ] {
        assert!(
            curve_surface_agreement::verify_correspondence(&c, &p, &s, &nodes, 1e-4, 100).is_err()
        );
    }
    for (tau, work) in [(0., 100), (f64::NAN, 100), (1e-4, 0), (1e-4, 100001)] {
        assert!(
            curve_surface_agreement::verify_correspondence(
                &c,
                &p,
                &s,
                &[[0., 0.], [1., 1.]],
                tau,
                work
            )
            .is_err()
        );
    }
}
