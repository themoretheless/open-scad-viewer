use super::*;
#[test]
fn outward_oblique_control_gap_proves_shear_and_refuses_touch_and_crossing() {
    let wall = |offset: f64| Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., offset], vec![0., 1., 1. + offset]],
            vec![vec![1., 0., 1. + offset], vec![1., 1., 2. + offset]],
        ],
        weights: vec![vec![1., 1.], vec![0.5, 0.5]],
        periodic_u: false,
        periodic_v: false,
    };
    let a = wall(0.);
    let b = wall(0.125);
    let domains = [[[0., 1.], [0., 1.]]; 2];
    let aa = rectangle_bounds(&a, domains[0]).unwrap();
    let bb = rectangle_bounds(&b, domains[1]).unwrap();
    assert!((0..3).all(|k| aa[k][0] <= bb[k][1] && bb[k][0] <= aa[k][1]));
    assert!(rectangle_control_gap(&a, &b, domains).unwrap().unwrap() > 0.);
    assert!(rectangle_control_gap(&b, &a, domains).unwrap().unwrap() > 0.);
    assert_eq!(rectangle_control_gap(&a, &a, domains).unwrap(), Some(0.));
    let mut crossing = b;
    for p in &mut crossing.control_points[0] {
        p[2] -= 0.25;
    }
    assert_eq!(
        rectangle_control_gap(&a, &crossing, domains).unwrap(),
        Some(0.)
    );
    let before = a.clone();
    rectangle_control_gap(&a, &crossing, domains).unwrap();
    assert_eq!(a, before);
}
fn plane(z: f64) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., z], vec![0., 10., z]],
            vec![vec![10., 0., z], vec![10., 10., z]],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn point(p: [f64; 3]) -> Surface {
    let mut s = plane(0.);
    for row in &mut s.control_points {
        for cp in row {
            *cp = p.to_vec();
        }
    }
    s
}
fn contains(r: &SurfaceDistance, exact: f64) {
    assert!(
        r.distance_interval_mm[0] <= exact && r.distance_interval_mm[1] >= exact,
        "exact {exact}, {r:?}"
    );
}
#[test]
fn planes_and_interior_point_projection() {
    let a = plane(0.);
    let b = plane(3.);
    let r = distance(&a, &b, 1e-8, 100).unwrap();
    contains(&r, 3.);
    assert!(r.converged);
    assert_eq!(r.cells, 1);
    let b = point([3.7, 6.2, 2.]);
    let r = distance(&a, &b, 1e-5, 10000).unwrap();
    contains(&r, 2.);
    assert!(r.converged, "{r:?}");
    assert!((r.parameters[0][0] - 0.37).abs() < 0.002);
    assert!((r.parameters[0][1] - 0.62).abs() < 0.002);
    for (i, s) in [&a, &b].into_iter().enumerate() {
        assert_eq!(
            r.points[i],
            s.evaluate(r.parameters[i][0], r.parameters[i][1])
                .unwrap()
                .point
        );
    }
}
#[test]
fn rational_cylinder_patch_and_weight_scaling() {
    for scale in [1e-11, 1., 1e11] {
        let mut a = plane(0.);
        a.degree_u = 2;
        a.knots_u = vec![0., 0., 0., 1., 1., 1.];
        a.control_points = vec![
            vec![vec![10., 0., 0.], vec![10., 0., 5.]],
            vec![vec![10., 10., 0.], vec![10., 10., 5.]],
            vec![vec![0., 10., 0.], vec![0., 10., 5.]],
        ];
        a.weights = vec![
            vec![scale; 2],
            vec![scale * std::f64::consts::FRAC_1_SQRT_2; 2],
            vec![scale; 2],
        ];
        let b = point([15., 15., 2.37]);
        let r = distance(&a, &b, 1e-3, 20000).unwrap();
        contains(&r, 450_f64.sqrt() - 10.);
        assert!(r.converged, "{r:?}");
        assert!((r.points[0][0].hypot(r.points[0][1]) - 10.).abs() < 1e-10);
    }
}
fn bowl() -> Surface {
    let c = [0.37, 0.62];
    let q = c.map(|x| [x * x, x * x - x, (1. - x) * (1. - x)]);
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![0., 0., 0., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 1., 1., 1.],
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| vec![i as f64 / 2., j as f64 / 2., q[0][i] + q[1][j]])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
#[test]
fn joint_minimum_inside_both_curved_surfaces() {
    let a = bowl();
    let mut b = a.clone();
    for row in &mut b.control_points {
        for p in row {
            p[2] = -p[2] - 2.;
        }
    }
    let r = distance(&a, &b, 0.002, 100000).unwrap();
    contains(&r, 2.);
    assert!(r.converged, "{r:?}");
    assert!(
        r.parameters
            .iter()
            .all(|uv| uv[0] > 0. && uv[0] < 1. && uv[1] > 0. && uv[1] < 1.)
    );
}
#[test]
fn work_limit_is_not_a_completed_minimum() {
    let a = plane(0.);
    let b = point([3.7, 6.2, 2.]);
    let r = distance(&a, &b, 1e-9, 1).unwrap();
    contains(&r, 2.);
    assert!(!r.converged);
    assert_eq!(r.reason, crate::DistanceStopReason::WorkLimit);
    assert_eq!(r.cells, 1);
}
#[test]
fn original_knots_multiple_spans_and_periodic_seam() {
    let mut a = plane(0.);
    a.knots_u = vec![0., 0., 0.4, 1., 1.];
    a.control_points
        .insert(1, vec![vec![4., 0., 0.], vec![4., 10., 0.]]);
    a.weights.insert(1, vec![1.; 2]);
    let b = point([7.3, 6.2, 2.]);
    let r = distance(&a, &b, 1e-4, 10000).unwrap();
    contains(&r, 2.);
    assert!(r.converged, "{r:?}");
    assert!(distance(&a, &b, 1e-3, 1).is_err());
    let mut ring = plane(0.);
    ring.periodic_u = true;
    ring.knots_u = vec![-1., 0., 1., 2., 3., 4., 5.];
    ring.control_points = vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.], [0., 0.]]
        .into_iter()
        .map(|p| vec![vec![p[0], p[1], 0.], vec![p[0], p[1], 2.]])
        .collect();
    ring.weights = vec![vec![1.; 2]; 5];
    let r = distance(&ring, &point([-1., 0., 0.]), 1e-8, 100).unwrap();
    contains(&r, 1.);
    assert!(r.converged);
    assert_eq!(r.parameters[0][0], 0.);
}
#[test]
fn translation_and_invalid_inputs() {
    let mut a = plane(0.);
    for row in &mut a.control_points {
        for p in row {
            p.iter_mut().for_each(|x| *x += 1e6);
        }
    }
    let r = distance(&a, &point([1e6 + 3.7, 1e6 + 6.2, 1e6 + 2.]), 0.001, 10000).unwrap();
    contains(&r, 2.);
    assert!(r.converged);
    assert!(distance(&a, &a, 0., 100).is_err());
    assert!(distance(&a, &a, 1e-3, 0).is_err());
    let mut bad = a.clone();
    bad.weights[0][0] = 0.;
    assert!(distance(&a, &bad, 0.001, 100).is_err());
}
#[test]
fn non_finite_boundaries_are_rejected_and_results_stay_finite() {
    let a = plane(0.);
    // nearest_point query point must be finite (1093).
    let err = nearest_point(&a, [f64::NAN, 0., 0.], 1e-6, 100).unwrap_err();
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    assert!(err.contains("point"), "{err}");
    // rectangle_bounds domain must be finite.
    let err = rectangle_bounds(&a, [[0., f64::INFINITY], [0., 1.]]).unwrap_err();
    assert_eq!(err.code, crate::INVALID_INPUT, "{err:?}");
    // A normal run returns finite witness points.
    let r = distance(&a, &point([3.7, 6.2, 2.]), 1e-5, 1000).unwrap();
    assert!(r.converged);
    assert!(r.points.iter().flatten().all(|v| v.is_finite()));
}
