use nurbs_core::{curve::Curve, curve_surface_distance::distance, surface::Surface};
fn panel() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![2., 2., 7., 7.],
        knots_v: vec![-3., -3., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 4., 0.]],
            vec![vec![4., 0., 0.], vec![4., 4., 0.]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}
fn line(a: [f64; 3], b: [f64; 3]) -> Curve {
    Curve {
        degree: 1,
        knots: vec![-2., -2., 5., 5.],
        control_points: vec![a.to_vec(), b.to_vec()],
        weights: vec![1., 1.],
        periodic: false,
    }
}
#[test]
fn analytic_full_domain_minima_and_witness_enclosures() {
    for (c, d) in [
        (line([0., 2., 3.], [4., 2., 3.]), 3.),
        (line([2., 2., -1.], [2., 2., 1.]), 0.),
        (line([-3., -4., 0.], [-3., -4., 2.]), 5.),
    ] {
        let r = distance(&c, &panel(), 1e-6, 1000).unwrap();
        assert!(r.converged, "{r:?}");
        assert!(
            r.distance_interval[0] <= d && d <= r.distance_interval[1],
            "{r:?}"
        );
        for (p, bs) in r.points.iter().zip(&r.point_enclosures) {
            for (x, b) in p.iter().zip(bs) {
                assert!(b[0] <= *x && *x <= b[1]);
            }
        }
    }
}
#[test]
fn budget_and_invalid_inputs_are_explicit() {
    let c = line([1.3, 1.7, -1.], [1.3, 1.7, 1.]);
    let s = panel();
    let r = distance(&c, &s, 1e-12, 1).unwrap();
    assert!(!r.converged);
    assert_eq!(r.distance_interval[0], 0.);
    for t in [0., -1., f64::NAN] {
        assert!(distance(&c, &s, t, 100).is_err());
    }
    for b in [0, 100001] {
        assert!(distance(&c, &s, 1e-4, b).is_err());
    }
    let mut flat = c.clone();
    for p in &mut flat.control_points {
        p.pop();
    }
    assert!(distance(&flat, &s, 1e-4, 100).is_err());
}
#[test]
fn disjoint_knot_spans_remain_covered() {
    let c =
        Curve::from_polyline(vec![vec![2., 2., 4.], vec![2., 2., 3.], vec![2., 2., 1.]]).unwrap();
    assert!(distance(&c, &panel(), 1e-6, 1).is_err());
    let r = distance(&c, &panel(), 1e-6, 1000).unwrap();
    assert!(r.converged);
    assert!(r.distance_interval[0] <= 1. && 1. <= r.distance_interval[1]);
}

#[test]
fn rational_quadratic_minimum_requires_subdivision() {
    let c = Curve {
        degree: 2,
        knots: vec![2., 2., 2., 7., 7., 7.],
        control_points: vec![vec![0., 2., 2.], vec![2., 2., 0.], vec![4., 2., 2.]],
        weights: vec![1., 2., 1.],
        periodic: false,
    };
    let r = distance(&c, &panel(), 1e-3, 10000).unwrap();
    assert!(r.converged, "{r:?}");
    let expected = 2. / 3.;
    assert!(
        r.distance_interval[0] <= expected && expected <= r.distance_interval[1],
        "{r:?}"
    );
    assert!(r.cells > 1);
    let limited = distance(&c, &panel(), 1e-12, 1).unwrap();
    assert!(!limited.converged);
    assert!(limited.distance_interval[0] <= expected && expected <= limited.distance_interval[1]);
}
