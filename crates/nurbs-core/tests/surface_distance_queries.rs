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

#[test]
fn nested_panels_have_interior_clearance_in_both_orders() {
    use nurbs_core::analysis::curve_measure::{inspect_surface_surface_clearance as inspect, ClearanceDecision::*};
    let mut outer = panel(0.);
    let mut inner = panel(3.);
    outer.weights = vec![vec![1.; 2]; 2];
    inner.weights = outer.weights.clone();
    for point in inner.control_points.iter_mut().flatten() {
        for x in &mut point[..2] { *x = 1. + *x * 0.5; }
    }
    for (a, b, outer_index) in [(&outer, &inner, 0), (&inner, &outer, 1)] {
        let (decision, report) = inspect(a, b, 3., 1e-5, 1000).unwrap();
        assert_eq!(decision, MeetsMinimum);
        let [u, v] = report.parameters[outer_index];
        assert!(2. < u && u < 7. && -3. < v && v < 1.);
        assert_eq!(inspect(a, b, 3.01, 1e-5, 1000).unwrap().0, BelowMinimum);
    }
    assert!(inspect(&inner, &outer, f64::NAN, 1e-5, 1000).is_err());
}

#[test]
fn trimmed_clearance_excludes_hole_and_requires_nonempty_faces() {
    use nurbs_core::{analysis::curve_measure::{inspect_trimmed_surface_clearance as inspect,
        ClearanceDecision::*}, curve::Curve, trim_domain::TrimDomain};
    let square = |a, b| vec![Curve::from_polyline(vec![vec![a,a], vec![b,a], vec![b,b],
        vec![a,b], vec![a,a]]).unwrap()];
    let full = TrimDomain::new(&[square(-1., 2.)], 1e-8).unwrap();
    let hole = square(0.25, 0.75)[0].reverse().unwrap();
    let ring = TrimDomain::new(&[square(-1., 2.), vec![hole]], 1e-8).unwrap();
    let empty = TrimDomain::new(&[square(2., 3.)], 1e-8).unwrap();
    let mut face = panel(0.);
    face.weights = vec![vec![1.;2];2];
    face.knots_u = vec![0.,0.,1.,1.];
    face.knots_v = face.knots_u.clone();
    let mut probe = face.clone();
    for p in probe.control_points.iter_mut().flatten() { *p = vec![2.,2.,0.]; }
    let run = |d, minimum| inspect((&face,d), (&probe,&full), minimum, 0.02, 10000, 500000);
    assert_eq!(run(&full, 0.5).unwrap().0, BelowMinimum);
    let (decision, report) = run(&ring, 0.5).unwrap();
    assert_eq!(decision, MeetsMinimum);
    let [u,v] = report.parameters.unwrap()[0];
    assert!((u-0.5).abs().max((v-0.5).abs()) >= 0.25);
    assert_eq!(run(&ring, 1.1).unwrap().0, BelowMinimum);
    assert_eq!(run(&empty, 0.).unwrap().0, Unresolved);
    assert!(run(&full, -1.).is_err());
}
