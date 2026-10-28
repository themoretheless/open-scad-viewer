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

fn bound(c: &ss::QualifiedGraphBoundaryRoots) -> ss::QualifiedGraphBoundaryBindings {
    ss::bind_graph_boundary_roots(c, 1000000).unwrap()
}
fn interior(b: &ss::GraphRootBinding) -> &ss::GraphRootEdgeClock {
    let ss::GraphRootLocation::Interior(ref edge) = b.location else {
        panic!("expected a strict original edge interior")
    };
    edge
}
fn cardinal(original: &ss::QualifiedQuadraticGraphSS, b: &ss::GraphRootBinding, quarter: usize) {
    let ss::GraphRootLocation::Vertex {
        original_vertex,
        ref incidences,
    } = b.location
    else {
        panic!("expected an exact original vertex")
    };
    assert_eq!(
        original.vertices()[original_vertex].point,
        ss::OriginalGraphSSPoint::EllipseCardinal(quarter)
    );
    assert_eq!(incidences.len(), 2);
    for port in incidences {
        let end = port.clock[0] as usize;
        assert_eq!(port.clock, [end as f64; 2]);
        assert_eq!(
            original.edges()[port.original_edge].vertices[end],
            original_vertex
        );
        assert!(
            original.branches()[port.original_branch]
                .edges
                .contains(&port.original_edge)
        );
    }
}
fn mixed_owner() -> ss::QualifiedQuadraticGraphSS {
    let mut a = surface(true, [[0., 1.], [0., 1.]]);
    let mut b = surface(false, [[0., 1.], [0., 1.]]);
    let q = [0.25, -0.25, 0.25];
    for i in 0..3 {
        for j in 0..3 {
            let u = i as f64 / 2.;
            let v = j as f64 / 2.;
            let common = if i == 2 && j == 2 { 1. } else { 0. };
            a.control_points[i][j][2] = q[i] + q[j] - (u - 0.5) * (v - 0.5) - 0.015625 + common;
            b.control_points[i][j][2] = common;
        }
    }
    let source = ss::FiberSources::new(&a, &b).unwrap();
    ss::qualify_quadratic_graph_ss(&source, Default::default()).unwrap()
}
fn factor_owner(crossing: bool) -> ss::QualifiedQuadraticGraphSS {
    let mut a = surface(true, [[0., 1.], [0., 1.]]);
    let b = surface(false, [[0., 1.], [0., 1.]]);
    let q = [0.25, -0.25, 0.25];
    for i in 0..3 {
        for j in 0..3 {
            a.control_points[i][j][2] = q[i] - if crossing { q[j] } else { 0.0625 };
        }
    }
    let source = ss::FiberSources::new(&a, &b).unwrap();
    ss::qualify_quadratic_graph_ss(&source, Default::default()).unwrap()
}
#[test]
fn original_quarter_inverse_binds_rational_boundary_roots_in_query_order() {
    let o = unit_owner();
    let c = boundary(vec![vec![0., 0.5625], vec![1., 0.5625]], vec![1., 2.]);
    let census = ss::qualify_graph_boundary_roots(&o, &c, 0, Default::default()).unwrap();
    let b = bound(&census);
    let cos = 0.9375f64.sqrt();
    let expected = [cos / 1.25, 0.25 / (1. + cos)];
    assert_eq!(b.bindings().len(), 2);
    for i in 0..2 {
        let edge = interior(&b.bindings()[i]);
        assert_eq!(b.bindings()[i].original_root, i);
        assert_eq!(
            o.edges()[edge.original_edge].geometry,
            ss::OriginalGraphSSEdgeGeometry::EllipseQuarter([1, 0][i])
        );
        contains(edge.clock, expected[i]);
        assert!(edge.clock[0] > 0. && edge.clock[1] < 1.);
    }
    b.reverify().unwrap();
}
#[test]
fn exact_cardinal_roots_keep_both_closed_loop_ports_without_snapping() {
    let o = unit_owner();
    let c = boundary(vec![vec![0., 0.5], vec![1., 0.5]], vec![1., 2.]);
    let census = ss::qualify_graph_boundary_roots(&o, &c, 0, Default::default()).unwrap();
    let b = bound(&census);
    cardinal(&o, &b.bindings()[0], 2);
    cardinal(&o, &b.bindings()[1], 0);
    b.reverify().unwrap();
}
#[test]
fn irrational_tilted_cardinal_vertices_follow_exact_carriers_and_original_incidence() {
    let o = mixed_owner();
    let c = boundary(vec![vec![0.25, 0.], vec![0.75, 1.]], vec![1., 2.]);
    let census = ss::qualify_graph_boundary_roots(&o, &c, 0, Default::default()).unwrap();
    let b = bound(&census);
    assert_eq!(b.bindings().len(), 2);
    cardinal(&o, &b.bindings()[0], 3);
    cardinal(&o, &b.bindings()[1], 1);
    let r = (1f64 / 48.).sqrt();
    for (i, s) in [-r, r].into_iter().enumerate() {
        contains(census.roots()[i].world[0], 0.5 + 0.5 * s);
        contains(census.roots()[i].world[1], 0.5 + s);
    }
    b.reverify().unwrap();
}
#[test]
fn mixed_basis_inverse_retains_original_quartic_world_height() {
    let o = mixed_owner();
    let c = boundary(vec![vec![0., 0.53125], vec![1., 0.53125]], vec![1.; 2]);
    let census = ss::qualify_graph_boundary_roots(&o, &c, 0, Default::default()).unwrap();
    let b = bound(&census);
    let delta = 0.0595703125f64.sqrt() / 2.;
    let radius = (1f64 / 48.).sqrt();
    for (i, s) in [-delta, delta].into_iter().enumerate() {
        let u = 0.5 + 0.015625 + s;
        let v = 0.53125;
        let x = s / 0.125;
        let y = 0.03125 / radius;
        let expected = if i == 0 { -x / (1. + y) } else { y / (1. + x) };
        contains(interior(&b.bindings()[i]).clock, expected);
        contains(census.roots()[i].world[2], u * u * v * v);
    }
    b.reverify().unwrap();
}
#[test]
fn distinct_query_events_on_one_ss_vertex_are_preserved_even_with_zero_query_chord() {
    let o = unit_owner();
    let c = boundary(
        vec![vec![0., 0.5], vec![1.75, 0.5], vec![0., 0.5]],
        vec![1.; 3],
    );
    let census = ss::qualify_graph_boundary_roots(&o, &c, 0, Default::default()).unwrap();
    let b = bound(&census);
    assert_eq!(b.bindings().len(), 4);
    for (i, q) in [2, 0, 0, 2].into_iter().enumerate() {
        assert_eq!(b.bindings()[i].original_root, i);
        cardinal(&o, &b.bindings()[i], q);
    }
    b.reverify().unwrap();
}
#[test]
fn near_cardinal_roots_inside_world_tolerance_remain_strict_interiors() {
    let o = unit_owner();
    let y = 0.5 + 2f64.powi(-30);
    let c = boundary(vec![vec![0., y], vec![1., y]], vec![1.; 2]);
    let census = ss::qualify_graph_boundary_roots(&o, &c, 0, Default::default()).unwrap();
    let b = bound(&census);
    assert!(census.roots().iter().all(|r| r.world[1][0] > 0.5));
    assert!(
        b.bindings()
            .iter()
            .all(|b| matches!(b.location, ss::GraphRootLocation::Interior(_)))
    );
    b.reverify().unwrap();
}
#[test]
fn complete_factor_graph_excludes_other_carriers_before_admitting_one_interior_edge() {
    for crossing in [false, true] {
        let o = factor_owner(crossing);
        let c = boundary(vec![vec![0., 0.5625], vec![1., 0.5625]], vec![1., 2.]);
        let census = ss::qualify_graph_boundary_roots(&o, &c, 0, Default::default()).unwrap();
        let b = bound(&census);
        let xs = if crossing {
            [0.4375, 0.5625]
        } else {
            [0.25, 0.75]
        };
        for (i, x) in xs.into_iter().enumerate() {
            let edge = interior(&b.bindings()[i]);
            let ss::OriginalGraphSSEdgeGeometry::Segment(p) =
                o.edges()[edge.original_edge].geometry
            else {
                panic!()
            };
            let axis = if p[0][0] != p[1][0] { 0 } else { 1 };
            let point = [x, 0.5625];
            contains(
                edge.clock,
                (point[axis] - p[0][axis]) / (p[1][axis] - p[0][axis]),
            );
            assert!(
                o.branches()[edge.original_branch]
                    .edges
                    .contains(&edge.original_edge)
            );
        }
        b.reverify().unwrap();
    }
}
#[test]
fn exact_natural_rail_roots_bind_owned_segment_end_vertices() {
    let o = factor_owner(false);
    let c = boundary(vec![vec![0., 0.], vec![1., 0.]], vec![1.; 2]);
    let census = ss::qualify_graph_boundary_roots(&o, &c, 0, Default::default()).unwrap();
    let b = bound(&census);
    for (i, x) in [0.25, 0.75].into_iter().enumerate() {
        let ss::GraphRootLocation::Vertex {
            original_vertex,
            ref incidences,
        } = b.bindings()[i].location
        else {
            panic!()
        };
        assert_eq!(
            o.vertices()[original_vertex].point,
            ss::OriginalGraphSSPoint::Normalized([x, 0.])
        );
        assert_eq!(incidences.len(), 1);
        let port = &incidences[0];
        assert_eq!(
            o.edges()[port.original_edge].vertices[port.clock[0] as usize],
            original_vertex
        );
    }
    b.reverify().unwrap();
}
#[test]
fn native_domain_and_support_swap_do_not_change_original_branch_clock_identity() {
    let o = owner([[[2., 6.], [-4., 4.]], [[-3., 5.], [7., 11.]]]);
    let c0 = boundary(vec![vec![2., 0.5], vec![6., 0.5]], vec![1.; 2]);
    let c1 = boundary(vec![vec![-3., 9.25], vec![5., 9.25]], vec![1.; 2]);
    let a = ss::qualify_graph_boundary_roots(&o, &c0, 0, Default::default()).unwrap();
    let b = ss::qualify_graph_boundary_roots(&o, &c1, 1, Default::default()).unwrap();
    let ba = bound(&a);
    let bb = bound(&b);
    for i in 0..2 {
        let a = interior(&ba.bindings()[i]);
        let b = interior(&bb.bindings()[i]);
        assert_eq!(a.original_edge, b.original_edge);
        assert!(a.clock[0] <= b.clock[1] && b.clock[0] <= a.clock[1]);
    }
    let sources = ss::FiberSources::new(o.source().second(), o.source().first()).unwrap();
    let swapped = ss::qualify_quadratic_graph_ss(&sources, Default::default()).unwrap();
    let r = ss::qualify_graph_boundary_roots(&swapped, &c1, 0, Default::default()).unwrap();
    let reverse = bound(&r);
    for i in 0..2 {
        assert_eq!(a.roots()[i].crossing_sign, -r.roots()[i].crossing_sign);
        assert_eq!(
            interior(&ba.bindings()[i]).original_edge,
            interior(&reverse.bindings()[i]).original_edge
        );
    }
    reverse.reverify().unwrap();
}
#[test]
fn binding_work_is_shared_exact_and_empty_census_stays_empty() {
    let o = unit_owner();
    let c = boundary(vec![vec![0., 0.5625], vec![1., 0.5625]], vec![1.; 2]);
    let census = ss::qualify_graph_boundary_roots(&o, &c, 0, Default::default()).unwrap();
    let b = bound(&census);
    let used = b.operations();
    assert!(ss::bind_graph_boundary_roots(&census, used).is_ok());
    assert!(ss::bind_graph_boundary_roots(&census, used - 1).is_err());
    assert!(ss::bind_graph_boundary_roots(&census, 0).is_err());
    assert!(ss::bind_graph_boundary_roots(&census, 1000001).is_err());
    let c = boundary(vec![vec![0., 0.875], vec![1., 0.875]], vec![1.; 2]);
    let empty = ss::qualify_graph_boundary_roots(&o, &c, 0, Default::default()).unwrap();
    let binding = ss::bind_graph_boundary_roots(&empty, 1).unwrap();
    assert!(binding.bindings().is_empty());
    assert_eq!(binding.operations(), 1);
}

#[test]
fn exact_native_domain_contacts_with_outside_poles_do_not_admit_a_tolerance_expansion() {
    let o = unit_owner();
    for (pole, weights) in [(2., vec![1.; 3]), (1.5, vec![1., 2., 1.])] {
        let c = boundary(vec![vec![0., 0.5], vec![pole, 0.5], vec![0., 0.5]], weights);
        // Both actual rational images lie in [0,1] and touch x=1 internally.
        let r = ss::qualify_graph_boundary_roots(&o, &c, 0, Default::default()).unwrap();
        assert_eq!(r.roots().len(), 4);
        let b = bound(&r);
        for (i, q) in [2, 0, 0, 2].into_iter().enumerate() {
            cardinal(&o, &b.bindings()[i], q);
        }
        let mut outside = c;
        outside.control_points[1][0] += 2f64.powi(-30);
        assert!(ss::qualify_graph_boundary_roots(&o, &outside, 0, Default::default()).is_err());
    }
}
#[test]
fn nonrepresentable_native_width_does_not_lose_an_exact_natural_rail_identity() {
    let original = factor_owner(false);
    let mut a = original.source().first().clone();
    let mut b = original.source().second().clone();
    a.knots_u = vec![1e-6, 1e-6, 1e-6, 1., 1., 1.];
    a.knots_v = vec![-4., -4., -4., 4., 4., 4.];
    b.knots_u = vec![2., 2., 2., 6., 6., 6.];
    b.knots_v = vec![7., 7., 7., 11., 11., 11.];
    let sources = ss::FiberSources::new(&a, &b).unwrap();
    let o = ss::qualify_quadratic_graph_ss(&sources, Default::default()).unwrap();
    let c = boundary(vec![vec![1e-6, -4.], vec![1., -4.]], vec![1., 2.]);
    let roots = ss::qualify_graph_boundary_roots(&o, &c, 0, Default::default()).unwrap();
    let b = bound(&roots);
    for (i, x) in [0.25, 0.75].into_iter().enumerate() {
        let ss::GraphRootLocation::Vertex {
            original_vertex,
            ref incidences,
        } = b.bindings()[i].location
        else {
            panic!()
        };
        assert_eq!(
            o.vertices()[original_vertex].point,
            ss::OriginalGraphSSPoint::Normalized([x, 0.])
        );
        assert_eq!(incidences.len(), 1);
    }
    b.reverify().unwrap();
}
#[test]
fn native_endpoint_facts_preserve_a_diagonal_vertex_carrier_across_nonrepresentable_widths() {
    let mut a = surface(true, [[1e-6, 1.], [3., 9.]]);
    let b = surface(false, [[-3., 5.], [7., 11.]]);
    let q = [0.25, -0.25, 0.25];
    for i in 0..3 {
        for j in 0..3 {
            let u = i as f64 / 2.;
            let v = j as f64 / 2.;
            a.control_points[i][j][2] = q[i] + 2. * q[j] - 2. * (u - 0.5) * (v - 0.5) - 0.015625;
        }
    }
    let sources = ss::FiberSources::new(&a, &b).unwrap();
    let o = ss::qualify_quadratic_graph_ss(&sources, Default::default()).unwrap();
    let c = boundary(vec![vec![1e-6, 3.], vec![1., 9.]], vec![1., 2.]);
    let roots = ss::qualify_graph_boundary_roots(&o, &c, 0, Default::default()).unwrap();
    let b = bound(&roots);
    cardinal(&o, &b.bindings()[0], 3);
    cardinal(&o, &b.bindings()[1], 1);
    b.reverify().unwrap();
}
