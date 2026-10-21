use nurbs_core::{curve::Curve, intersection::certified_curve_surface, surface::Surface};
fn surface(curved: bool) -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 1., 0.]],
            vec![
                vec![1., 0., 0.],
                vec![1., 1., if curved { 0.1 } else { 0. }],
            ],
        ],
        weights: vec![vec![1.; 2]; 2],
        periodic_u: false,
        periodic_v: false,
    }
}
fn line(x: f64, z0: f64, z1: f64) -> Curve {
    Curve::from_polyline(vec![vec![x, 0.4, z0], vec![x, 0.4, z1]]).unwrap()
}
#[test]
fn original_curve_curved_surface_has_certified_interior_root_and_tight_enclosure() {
    let c = line(0.3, -0.5, 0.5);
    let s = surface(true);
    let before = (c.clone(), s.clone());
    let r = certified_curve_surface::intersections(&c, &s, 1000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.roots.len(), 1);
    let root = &r.roots[0];
    for (box_, value) in root.parameter_box.iter().zip([0.512, 0.3, 0.4]) {
        assert!(box_[0] <= value && value <= box_[1], "{box_:?} {value}");
        assert!(box_[1] - box_[0] < 1e-10, "{box_:?}");
    }
    assert_eq!((c, s), before);
}
#[test]
fn rational_weights_on_both_original_sources_are_included_in_root_proof() {
    let mut s = surface(false);
    s.weights = vec![vec![1., 1.125], vec![1.0625, 1.]];
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![0.3, 0.4, -0.5],
            vec![0.3, 0.4, 0.],
            vec![0.3, 0.4, 0.5],
        ],
        weights: vec![1., 1.0625, 1.],
        periodic: false,
    };
    let r = certified_curve_surface::intersections(&c, &s, 1000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.roots.len(), 1);
    assert!(r.roots[0].parameter_box[0][0] <= 0.5 && 0.5 <= r.roots[0].parameter_box[0][1]);
    for (box_, value) in r.roots[0].position_bounds.iter().zip([0.3, 0.4, 0.]) {
        assert!(box_[0] <= value && value <= box_[1]);
        assert!(box_[1] - box_[0] < 1e-10);
    }
}
#[test]
fn all_original_knot_spans_and_nonunit_domains_are_covered() {
    let mut c = Curve::from_polyline(vec![
        vec![0.3, 0.4, -0.6],
        vec![0.3, 0.4, 0.3],
        vec![0.3, 0.4, 1.],
    ])
    .unwrap();
    for t in &mut c.knots {
        *t = 0.3 + *t * 1.4;
    }
    let r = certified_curve_surface::intersections(&c, &surface(true), 1000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.roots.len(), 1);
    assert!(r.excluded > 0);
    assert!(r.roots[0].parameter_box[0][0] > c.domain()[0]);
}
#[test]
fn boundary_tangency_coincidence_and_budget_never_disappear_as_empty_complete() {
    let s = surface(false);
    let tangent = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![0.3, 0.4, 0.25],
            vec![0.3, 0.4, -0.25],
            vec![0.3, 0.4, 0.25],
        ],
        weights: vec![1.; 3],
        periodic: false,
    };
    for c in [line(0., -0.5, 0.5), tangent, line(0.3, 0., 0.)] {
        let r = certified_curve_surface::intersections(&c, &s, 64).unwrap();
        assert!(!r.complete, "{r:?}");
        assert!(!r.unresolved.is_empty());
        assert!(r.roots.is_empty());
        assert!(r.cells <= 64);
    }
    let r = certified_curve_surface::intersections(&line(0.3, -0.5, 0.5), &s, 1).unwrap();
    assert!(r.cells <= 1); // A strict affine inclusion may finish in one cell.
    assert!(r.complete || !r.unresolved.is_empty());
    assert!(certified_curve_surface::intersections(&line(0.3, -0.5, 0.5), &s, 0).is_err());
}
#[test]
fn separated_sources_have_complete_empty_root_cover() {
    let r = certified_curve_surface::intersections(&line(2., -0.5, 0.5), &surface(false), 1000)
        .unwrap();
    assert!(r.complete && r.roots.is_empty());
    assert!(r.excluded > 0);
}

#[test]
fn cubic_rational_surface_and_nonplanar_cubic_curve_use_original_coefficients() {
    let mut s = Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: (0..4)
            .map(|i| {
                (0..4)
                    .map(|j| {
                        let u = i as f64 / 3.;
                        let v = j as f64 / 3.;
                        vec![u, v, 0.1 * u * v + if i == 1 && j == 2 { 0.01 } else { 0. }]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    for i in 0..4 {
        for j in 0..4 {
            s.weights[i][j] = 1. + 0.015625 * ((i + j) % 3) as f64;
        }
    }
    let p = s.evaluate(0.3, 0.4).unwrap().point;
    let c = Curve {
        degree: 3,
        knots: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: vec![
            vec![p[0] - 0.03, p[1] - 0.02, p[2] - 0.5],
            vec![p[0] + 0.01, p[1] - 0.005, p[2] - 1. / 6.],
            vec![p[0] - 0.01, p[1] + 0.005, p[2] + 1. / 6.],
            vec![p[0] + 0.03, p[1] + 0.02, p[2] + 0.5],
        ],
        weights: vec![1., 1.0625, 1.0625, 1.],
        periodic: false,
    };
    let r = certified_curve_surface::intersections(&c, &s, 1000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.roots.len(), 1);
    for (range, near) in r.roots[0].parameter_box.iter().zip([0.5, 0.3, 0.4]) {
        assert!(
            range[0] <= near + 1e-10 && near - 1e-10 <= range[1],
            "{range:?}"
        );
        assert!(range[1] - range[0] < 1e-9, "{range:?}");
    }
}

#[test]
fn oriented_crossing_is_proved_and_changes_with_original_parameter_orientation() {
    let c = line(0.3, -0.5, 0.5);
    let s = surface(true);
    for (reverse_curve, reverse_surface, expected) in [
        (false, false, 1),
        (true, false, -1),
        (false, true, -1),
        (true, true, 1),
    ] {
        let curve = if reverse_curve {
            c.reverse().unwrap()
        } else {
            c.clone()
        };
        let mut support = s.clone();
        if reverse_surface {
            support.control_points.reverse();
            support.weights.reverse();
        }
        let r = certified_curve_surface::intersections(&curve, &support, 1000).unwrap();
        assert!(r.complete, "{r:?}");
        assert_eq!(r.roots.len(), 1);
        assert_eq!(r.roots[0].crossing_sign, expected);
    }
}

#[test]
fn distinct_parameter_roots_are_retained_even_at_the_same_world_position() {
    let c = Curve {
        degree: 2,
        knots: vec![0., 0., 0., 1., 1., 1.],
        control_points: vec![
            vec![0.3, 0.4, 0.16],
            vec![0.3, 0.4, -0.34],
            vec![0.3, 0.4, 0.16],
        ],
        weights: vec![1.; 3],
        periodic: false,
    };
    let r = certified_curve_surface::intersections(&c, &surface(false), 1000).unwrap();
    assert!(r.complete, "{r:?}");
    assert_eq!(r.roots.len(), 2);
    assert_eq!(r.roots[0].crossing_sign, -1);
    assert_eq!(r.roots[1].crossing_sign, 1);
    for (root, t) in r.roots.iter().zip([0.2, 0.8]) {
        assert!(root.parameter_box[0][0] <= t + 1e-10 && t - 1e-10 <= root.parameter_box[0][1]);
        assert!(
            root.parameter_box[0][1] - root.parameter_box[0][0] < 1e-9,
            "{root:?} cells={}",
            r.cells
        );
    }
}
