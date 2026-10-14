use nurbs_core::{
    surface::Surface,
    surface_distance::{distance, nearest_point},
};
fn panel(z: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., z], vec![0., 4., z]],
            vec![vec![4., 0., z], vec![4., 4., z]],
        ],
        weights: vec![vec![1., 2.], vec![3., 6.]],
        periodic_u: false,
        periodic_v: false,
    }
}
fn contains(b: [f64; 2], x: f64) {
    assert!(b[0] <= x && x <= b[1], "{x} outside {b:?}");
}
#[test]
fn rational_panel_queries_have_analytic_minima() {
    for (q, d) in [([2., 2., 3.], 3.), ([-3., -4., 0.], 5.), ([7., 2., 4.], 5.)] {
        let r = nearest_point(&panel(0.), q, 1e-4, 10000).unwrap();
        assert!(r.converged, "{r:?}");
        contains(r.distance_interval, d);
        for (x, b) in r.point.iter().zip(&r.point_enclosure) {
            contains(*b, *x);
        }
    }
}
#[test]
fn full_panel_distances_and_reversed_arguments() {
    for (a, b, d) in [(panel(0.), panel(3.), 3.), (panel(0.), panel(0.), 0.)] {
        for (x, y) in [(&a, &b), (&b, &a)] {
            let r = distance(x, y, 1e-4, 10000).unwrap();
            assert!(r.converged, "{r:?}");
            contains(r.distance_interval_mm, d);
        }
    }
}
#[test]
fn work_limit_and_invalid_queries_remain_explicit() {
    let s = panel(0.);
    let r = nearest_point(&s, [1.3, 1.7, 3.], 1e-12, 1).unwrap();
    assert!(!r.converged);
    contains(r.distance_interval, 3.);
    assert!(nearest_point(&s, [f64::NAN, 0., 0.], 1e-4, 100).is_err());
    for t in [0., -1., f64::INFINITY] {
        assert!(nearest_point(&s, [0.; 3], t, 100).is_err());
        assert!(distance(&s, &s, t, 100).is_err());
    }
    for b in [0, 100001] {
        assert!(nearest_point(&s, [0.; 3], 1e-4, b).is_err());
        assert!(distance(&s, &s, 1e-4, b).is_err());
    }
}
