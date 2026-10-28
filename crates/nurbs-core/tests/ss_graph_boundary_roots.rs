use nurbs_core::{curve::Curve, ss_intersection as ss, surface::Surface};
fn surface(circle: bool, domain: [[f64; 2]; 2]) -> Surface {
    let q = [0.25, -0.25, 0.25];
    Surface {
        degree_u: 2,
        degree_v: 2,
        knots_u: vec![domain[0][0]; 3]
            .into_iter()
            .chain(vec![domain[0][1]; 3])
            .collect(),
        knots_v: vec![domain[1][0]; 3]
            .into_iter()
            .chain(vec![domain[1][1]; 3])
            .collect(),
        control_points: (0..3)
            .map(|i| {
                (0..3)
                    .map(|j| {
                        vec![
                            i as f64 / 2.,
                            j as f64 / 2.,
                            if circle { q[i] + q[j] - 0.0625 } else { 0. },
                        ]
                    })
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
fn owner(domains: [[[f64; 2]; 2]; 2]) -> ss::QualifiedQuadraticGraphSS {
    let s = ss::FiberSources::new(&surface(true, domains[0]), &surface(false, domains[1])).unwrap();
    ss::qualify_quadratic_graph_ss(&s, Default::default()).unwrap()
}
fn unit_owner() -> ss::QualifiedQuadraticGraphSS {
    owner([[[0., 1.], [0., 1.]]; 2])
}
fn boundary(points: Vec<Vec<f64>>, weights: Vec<f64>) -> Curve {
    let degree = points.len() - 1;
    Curve {
        degree,
        knots: vec![10.; degree + 1]
            .into_iter()
            .chain(vec![14.; degree + 1])
            .collect(),
        control_points: points,
        weights,
        periodic: false,
    }
}
fn contains(b: [f64; 2], x: f64) {
    assert!(b[0] <= x && x <= b[1], "{b:?} does not contain {x}");
}
#[test]
fn original_circle_has_two_simple_roots_on_a_rational_nonuniform_trim_clock() {
    let o = unit_owner();
    let c = boundary(vec![vec![0., 0.5625], vec![1., 0.5625]], vec![1., 2.]);
    let saved = c.clone();
    let r = ss::qualify_graph_boundary_roots(&o, &c, 0, Default::default()).unwrap();
    assert_eq!(r.roots().len(), 2);
    let radius = (0.0625f64 - 0.0625 * 0.0625).sqrt();
    for (i, x) in [0.5 - radius, 0.5 + radius].into_iter().enumerate() {
        let t = x / (2. - x);
        let root = &r.roots()[i];
        contains(root.clock, t);
        contains(root.boundary_parameter, 10. + 4. * t);
        contains(root.world[0], x);
        contains(root.world[1], 0.5625);
        contains(root.world[2], 0.);
        assert_eq!(root.crossing_sign, if i == 0 { -1 } else { 1 });
    }
    assert!(r.roots()[0].clock[1] < r.roots()[1].clock[0]);
    r.reverify().unwrap();
    assert_eq!(c, saved);
}
#[test]
fn four_nonmonotone_original_roots_are_not_reduced_to_endpoint_or_chord_crossings() {
    let o = unit_owner();
    let c = boundary(
        vec![
            vec![0.25, 0.453125],
            vec![0.5, -0.046875],
            vec![0.75, 0.453125],
        ],
        vec![1.; 3],
    );
    let r = ss::qualify_graph_boundary_roots(&o, &c, 0, Default::default()).unwrap();
    assert_eq!(r.roots().len(), 4);
    let a = 0.02734375f64.sqrt();
    let b = 0.05859375f64.sqrt();
    for (i, s) in [-b, -a, a, b].into_iter().enumerate() {
        contains(r.roots()[i].clock, 0.5 + 2. * s);
        contains(r.roots()[i].world[0], 0.5 + s);
        contains(r.roots()[i].world[1], 0.203125 + 4. * s * s);
    }
    r.reverify().unwrap();
}
#[test]
fn both_original_native_domains_map_the_same_boundary_roots_without_source_replacement() {
    let o = owner([[[2., 6.], [-4., 4.]], [[-3., 5.], [7., 11.]]]);
    let first = boundary(vec![vec![2., 0.5], vec![6., 0.5]], vec![1.; 2]);
    let second = boundary(vec![vec![-3., 9.25], vec![5., 9.25]], vec![1.; 2]);
    let a = ss::qualify_graph_boundary_roots(&o, &first, 0, Default::default()).unwrap();
    let b = ss::qualify_graph_boundary_roots(&o, &second, 1, Default::default()).unwrap();
    assert_eq!(a.roots().len(), 2);
    assert_eq!(b.roots().len(), 2);
    let radius = (0.0625f64 - 0.0625 * 0.0625).sqrt();
    for (i, x) in [0.5 - radius, 0.5 + radius].into_iter().enumerate() {
        for r in [&a, &b] {
            contains(r.roots()[i].clock, x);
            contains(r.roots()[i].native_uv[0][0], 2. + 4. * x);
            contains(r.roots()[i].native_uv[1][0], -3. + 8. * x);
            contains(r.roots()[i].world[0], x);
        }
    }
    a.reverify().unwrap();
    b.reverify().unwrap();
}
#[test]
fn exclusions_have_complete_empty_census_but_tangencies_and_endpoint_events_are_not_erased() {
    let o = unit_owner();
    let away = boundary(vec![vec![0., 0.875], vec![1., 0.875]], vec![1.; 2]);
    assert!(
        ss::qualify_graph_boundary_roots(&o, &away, 0, Default::default())
            .unwrap()
            .roots()
            .is_empty()
    );
    for c in [
        boundary(vec![vec![0., 0.75], vec![1., 0.75]], vec![1.; 2]),
        boundary(vec![vec![0.25, 0.5], vec![0.75, 0.5]], vec![1.; 2]),
    ] {
        assert!(
            ss::qualify_graph_boundary_roots(
                &o,
                &c,
                0,
                ss::GraphBoundaryRootLimits {
                    max_operations: 30000,
                    max_depth: 12,
                    ..Default::default()
                }
            )
            .is_err()
        );
    }
}
#[test]
fn actual_boundary_domain_and_shared_operation_limits_are_mandatory() {
    let o = unit_owner();
    let c = boundary(vec![vec![0., 0.5625], vec![1., 0.5625]], vec![1.; 2]);
    let r = ss::qualify_graph_boundary_roots(&o, &c, 0, Default::default()).unwrap();
    let used = r.operations();
    assert!(
        ss::qualify_graph_boundary_roots(
            &o,
            &c,
            0,
            ss::GraphBoundaryRootLimits {
                max_operations: used,
                ..Default::default()
            }
        )
        .is_ok()
    );
    assert!(
        ss::qualify_graph_boundary_roots(
            &o,
            &c,
            0,
            ss::GraphBoundaryRootLimits {
                max_operations: used - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut outside = c.clone();
    outside.control_points[0][0] = -0.125;
    assert!(ss::qualify_graph_boundary_roots(&o, &outside, 0, Default::default()).is_err());
    assert!(ss::qualify_graph_boundary_roots(&o, &c, 2, Default::default()).is_err());
    assert!(
        ss::qualify_graph_boundary_roots(
            &o,
            &c,
            0,
            ss::GraphBoundaryRootLimits {
                world_tolerance: f64::NAN,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut partial = c.clone();
    partial.knots[0] = 9.;
    assert!(ss::qualify_graph_boundary_roots(&o, &partial, 0, Default::default()).is_err());
}
