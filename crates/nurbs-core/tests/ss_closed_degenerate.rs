use nurbs_core::{
    ss_intersection::{self as ss, QuadraticGraphSSKind as K},
    surface::Surface,
};
fn graph(a: f64, b: f64, k: f64, domain: [[f64; 2]; 2]) -> Surface {
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
                    .map(|j| vec![i as f64 / 2., j as f64 / 2., a * q[i] + b * q[j] + k])
                    .collect()
            })
            .collect(),
        weights: vec![vec![1.; 3]; 3],
        periodic_u: false,
        periodic_v: false,
    }
}
fn pair(a: f64, b: f64, k: f64) -> ss::FiberSources {
    ss::FiberSources::new(
        &graph(a, b, k, [[0., 1.]; 2]),
        &graph(0., 0., 0., [[0., 1.]; 2]),
    )
    .unwrap()
}
fn own(s: &ss::FiberSources) -> ss::QualifiedQuadraticGraphSS {
    ss::qualify_quadratic_graph_ss(s, Default::default()).unwrap()
}
fn contains(interval: [f64; 2], value: f64) {
    assert!(
        interval[0] <= value && value <= interval[1],
        "{interval:?} misses {value}"
    );
}
#[test]
fn closed_circle_is_one_complete_cycle_with_four_exact_shared_vertices_and_two_trims_per_edge() {
    let o = own(&pair(1., 1., -0.0625));
    assert_eq!(o.kind(), K::ClosedEllipse);
    assert!(o.covers_entire_surfaces());
    assert_eq!(o.branches().len(), 1);
    assert!(o.branches()[0].closed);
    assert_eq!(o.vertices().len(), 4);
    assert_eq!(o.edges().len(), 4);
    assert_eq!(o.trims().len(), 8);
    let points = [[0.75, 0.5], [0.5, 0.75], [0.25, 0.5], [0.5, 0.25]];
    for i in 0..4 {
        assert_eq!(o.edges()[i].vertices, [i, (i + 1) % 4]);
        for t in [0., 0.5, 1.] {
            let p = o.edge_station(i, t, 1024).unwrap();
            contains(p.world[2], 0.);
            if t == 0. {
                for a in 0..2 {
                    contains(p.normalized_uv[a], points[i][a]);
                }
            }
            if t == 1. {
                for a in 0..2 {
                    contains(p.normalized_uv[a], points[(i + 1) % 4][a]);
                }
            }
        }
        let p = o.vertex_station(i, 1024).unwrap();
        for a in 0..2 {
            contains(p.world[a], points[i][a]);
        }
    }
    let p = o.edge_station(0, 0.5, 1024).unwrap();
    contains(p.world[0], 0.65);
    contains(p.world[1], 0.7);
    o.reverify().unwrap();
}
#[test]
fn irrational_ellipse_radii_retain_algebraic_stations_in_both_original_native_domains() {
    let s = ss::FiberSources::new(
        &graph(2., 1., -0.0625, [[2., 4.], [5., 9.]]),
        &graph(0., 0., 0., [[-3., 1.], [10., 12.]]),
    )
    .unwrap();
    let o = own(&s);
    assert_eq!(o.kind(), K::ClosedEllipse);
    let p = o.vertex_station(0, 1024).unwrap();
    let u = 0.5 + (0.03125f64).sqrt();
    contains(p.world[0], u);
    contains(p.world[1], 0.5);
    contains(p.world[2], 0.);
    contains(p.first_uv[0], 2. + 2. * u);
    contains(p.second_uv[0], -3. + 4. * u);
    contains(p.first_uv[1], 7.);
    contains(p.second_uv[1], 11.);
    o.reverify().unwrap();
}
#[test]
fn isolated_tangent_point_has_no_fake_branch_or_trim_curve() {
    let o = own(&pair(1., 1., 0.));
    assert_eq!(o.kind(), K::IsolatedTangentPoint);
    assert_eq!(o.vertices().len(), 1);
    assert!(o.vertices()[0].singular);
    assert!(o.edges().is_empty() && o.branches().is_empty() && o.trims().is_empty());
    let p = o.vertex_station(0, 1024).unwrap();
    for a in 0..2 {
        contains(p.world[a], 0.5);
    }
    contains(p.world[2], 0.);
    o.reverify().unwrap();
}
#[test]
fn tangent_double_lines_are_complete_multiplicity_two_curves_in_either_axis() {
    for (a, b, axis) in [(1., 0., 0), (0., 1., 1), (-1., 0., 0)] {
        let o = own(&pair(a, b, 0.));
        assert_eq!(o.kind(), K::TangentDoubleLine);
        assert_eq!(o.branches().len(), 1);
        assert!(!o.branches()[0].closed);
        assert_eq!(o.edges()[0].multiplicity, 2);
        for t in [0., 0.25, 0.5, 1.] {
            let p = o.edge_station(0, t, 1024).unwrap();
            contains(p.world[axis], 0.5);
            contains(p.world[1 - axis], t);
            contains(p.world[2], 0.);
        }
        o.reverify().unwrap();
    }
}
#[test]
fn crossing_node_has_four_incident_halfedges_and_two_distinct_original_branches() {
    let o = own(&pair(1., -1., 0.));
    assert_eq!(o.kind(), K::CrossingLines);
    assert_eq!(o.branches().len(), 2);
    assert_eq!(o.edges().len(), 4);
    assert_eq!(o.vertices().len(), 5);
    assert!(o.vertices()[0].singular);
    assert_eq!(
        o.edges().iter().filter(|e| e.vertices.contains(&0)).count(),
        4
    );
    for e in 0..4 {
        for t in [0., 0.5, 1.] {
            let p = o.edge_station(e, t, 1024).unwrap();
            contains(p.world[2], 0.);
        }
    }
    o.reverify().unwrap();
}
#[test]
fn exact_critical_levels_never_merge_tiny_closed_loops_empty_sets_or_tangent_points() {
    let tiny = 2f64.powi(-40);
    for (k, expected) in [
        (-tiny, K::ClosedEllipse),
        (0., K::IsolatedTangentPoint),
        (tiny, K::Empty),
    ] {
        assert_eq!(own(&pair(1., 1., k)).kind(), expected);
    }
    let o = own(&pair(0., 0., 0.));
    assert_eq!(o.kind(), K::CoincidentPatch);
    assert!(o.branches().is_empty());
    let p = o.overlap_station([0.125, 0.875], 1024).unwrap();
    contains(p.world[0], 0.125);
    contains(p.world[1], 0.875);
    assert_eq!(own(&pair(0., 0., 0.125)).kind(), K::Empty);
}
#[test]
fn unsupported_source_identities_and_exhausted_work_never_publish_false_topology() {
    for s in [pair(1., 1., -1.), pair(1., -1., 0.125), pair(2., -1., 0.)] {
        assert!(ss::qualify_quadratic_graph_ss(&s, Default::default()).is_err());
    }
    let s = pair(1., 1., -0.0625);
    for max in [0, 1, 4097] {
        assert!(
            ss::qualify_quadratic_graph_ss(
                &s,
                ss::QuadraticGraphSSLimits {
                    max_operations: max
                }
            )
            .is_err()
        );
    }
    let mut a = s.first().clone();
    a.weights[1][1] = 2.;
    assert!(
        ss::qualify_quadratic_graph_ss(
            &ss::FiberSources::new(&a, s.second()).unwrap(),
            Default::default()
        )
        .is_err()
    );
    let mut a = s.first().clone();
    a.control_points[1][1][0] += 2f64.powi(-40);
    assert!(
        ss::qualify_quadratic_graph_ss(
            &ss::FiberSources::new(&a, s.second()).unwrap(),
            Default::default()
        )
        .is_err()
    );
    let o = own(&s);
    for t in [f64::NAN, f64::INFINITY, -0.01, 1.01] {
        assert!(o.edge_station(0, t, 1024).is_err());
    }
    assert!(o.edge_station(4, 0.5, 1024).is_err());
    assert!(o.edge_station(0, 0.5, 1).is_err());
    assert!(o.vertex_station(4, 1024).is_err());
    assert!(o.overlap_station([0.5; 2], 1024).is_err());
}

#[path = "support/ss_discovered_sources.rs"]
mod regular_fixture;
#[test]
fn public_dispatch_uses_complete_exact_graph_owners_and_preserves_regular_original_arc_fallback() {
    for (s, expected) in [
        (pair(1., 1., -0.0625), K::ClosedEllipse),
        (pair(1., 1., 0.), K::IsolatedTangentPoint),
        (pair(1., 0., 0.), K::TangentDoubleLine),
        (pair(1., -1., 0.), K::CrossingLines),
    ] {
        let o = ss::construct_qualified_ss_branches(&s, Default::default()).unwrap();
        assert!(o.covers_entire_surfaces());
        match &o {
            ss::QualifiedSurfaceIntersection::ClosedOrDegenerateGraph(q) => {
                assert_eq!(q.kind(), expected)
            }
            _ => panic!("Expected source-exact graph admission"),
        };
        o.reverify().unwrap();
    }
    let source = regular_fixture::sources(&[0.1875, -0.3125, 0.1875], 0.);
    let o = ss::construct_qualified_ss_branches(&source, Default::default()).unwrap();
    match &o {
        ss::QualifiedSurfaceIntersection::RegularOpenArcs(q) => assert_eq!(q.branches().len(), 2),
        _ => panic!("Expected full original-root discovery fallback"),
    };
    o.reverify().unwrap();
}
#[test]
fn crossing_edge_stations_retain_independent_diagonal_geometry_and_true_shared_singular_node() {
    let o = own(&pair(1., -1., 0.));
    for (e, expected) in [[0.25, 0.25], [0.75, 0.75], [0.25, 0.75], [0.75, 0.25]]
        .into_iter()
        .enumerate()
    {
        let p = o.edge_station(e, 0.5, 1024).unwrap();
        for a in 0..2 {
            contains(p.world[a], expected[a]);
        }
    }
}

#[test]
fn dispatcher_never_masks_exact_work_exhaustion_with_a_larger_marching_or_discovery_budget() {
    let e = ss::construct_qualified_ss_branches(
        &pair(1., 1., -0.0625),
        ss::QualifiedSSBuildLimits {
            quadratic: ss::QuadraticGraphSSLimits { max_operations: 1 },
            ..Default::default()
        },
    )
    .unwrap_err();
    assert_eq!(e.code, "NURBS_RESOURCE_LIMIT");
}

#[test]
fn unequal_exact_crossing_slopes_own_complete_original_branches_and_native_clocks() {
    for (a, b) in [
        (4., -1.),
        (1., -4.),
        (-4., 1.),
        (-1., 4.),
        (9., -16.),
        (-9., 16.),
    ] {
        let source = ss::FiberSources::new(
            &graph(a, b, 0., [[2., 6.], [-4., 4.]]),
            &graph(0., 0., 0., [[-2., 0.], [8., 16.]]),
        )
        .unwrap();
        let saved = (source.first().clone(), source.second().clone());
        let owner = own(&source);
        assert_eq!(owner.kind(), K::CrossingLines);
        assert!(owner.covers_entire_surfaces());
        assert_eq!(
            (
                owner.vertices().len(),
                owner.edges().len(),
                owner.branches().len(),
                owner.trims().len()
            ),
            (5, 4, 2, 8)
        );
        assert_eq!(
            owner
                .edges()
                .iter()
                .filter(|e| e.vertices.contains(&0))
                .count(),
            4
        );
        for (edge_index, edge) in owner.edges().iter().enumerate() {
            assert_eq!(edge.multiplicity, 1);
            for t in [0., 0.125, 0.5, 0.875, 1.] {
                let station = owner.edge_station(edge_index, t, 1024).unwrap();
                let xy = station.normalized_uv.map(|v| v[0] * 0.5 + v[1] * 0.5);
                let residual = a * (xy[0] - 0.5).powi(2) + b * (xy[1] - 0.5).powi(2);
                assert!(
                    residual.abs() < 1e-12,
                    "independent conic equation: {residual}"
                );
                for axis in 0..2 {
                    assert!(
                        station.normalized_uv[axis][0] >= -1e-12
                            && station.normalized_uv[axis][1] <= 1. + 1e-12
                    );
                    contains(station.world[axis], xy[axis]);
                }
                contains(station.first_uv[0], 2. + 4. * xy[0]);
                contains(station.first_uv[1], -4. + 8. * xy[1]);
                contains(station.second_uv[0], -2. + 2. * xy[0]);
                contains(station.second_uv[1], 8. + 8. * xy[1]);
                contains(station.world[2], 0.);
            }
        }
        owner.reverify().unwrap();
        let public = ss::construct_qualified_ss_branches(&source, Default::default()).unwrap();
        assert!(matches!(
            public,
            ss::QualifiedSurfaceIntersection::ClosedOrDegenerateGraph(_)
        ));
        public.reverify().unwrap();
        let limits = ss::QuadraticGraphSSLimits {
            max_operations: owner.operations() - 1,
        };
        assert_eq!(
            ss::qualify_quadratic_graph_ss(&source, limits)
                .unwrap_err()
                .code,
            "NURBS_RESOURCE_LIMIT"
        );
        assert_eq!(
            ss::construct_qualified_ss_branches(
                &source,
                ss::QualifiedSSBuildLimits {
                    quadratic: limits,
                    ..Default::default()
                }
            )
            .unwrap_err()
            .code,
            "NURBS_RESOURCE_LIMIT"
        );
        assert_eq!((source.first(), source.second()), (&saved.0, &saved.1));
    }
}

#[test]
fn exact_parallel_factors_keep_each_original_component_and_refuse_unrepresentable_roots() {
    for (a, b, k, axis) in [
        (1., 0., -0.0625, 0),
        (0., 4., -0.25, 1),
        (-1., 0., 0.0625, 0),
        (1., 0., -0.25, 0),
        (2f64.powi(-40), 0., -2f64.powi(-80), 0),
    ] {
        let source = pair(a, b, k);
        let owner = own(&source);
        assert_eq!(owner.kind(), K::ParallelLineFactors);
        assert_eq!(
            (
                owner.vertices().len(),
                owner.edges().len(),
                owner.branches().len(),
                owner.trims().len()
            ),
            (4, 2, 2, 4)
        );
        assert!(owner.vertices().iter().all(|v| !v.singular));
        let radius = (-k / if a != 0. { a } else { b }).sqrt();
        for (edge, sign) in [-1., 1.].into_iter().enumerate() {
            assert_eq!(owner.edges()[edge].multiplicity, 1);
            assert!(!owner.branches()[edge].closed);
            for t in [0., 0.125, 0.5, 1.] {
                let station = owner.edge_station(edge, t, 1024).unwrap();
                contains(station.world[axis], 0.5 + sign * radius);
                contains(station.world[1 - axis], t);
                contains(station.world[2], 0.);
            }
        }
        owner.reverify().unwrap();
        let public = ss::construct_qualified_ss_branches(&source, Default::default()).unwrap();
        assert!(matches!(
            public,
            ss::QualifiedSurfaceIntersection::ClosedOrDegenerateGraph(_)
        ));
        let limits = ss::QuadraticGraphSSLimits {
            max_operations: owner.operations() - 1,
        };
        assert_eq!(
            ss::qualify_quadratic_graph_ss(&source, limits)
                .unwrap_err()
                .code,
            "NURBS_RESOURCE_LIMIT"
        );
    }
    // This input offset is lost while authoring binary64 controls. The owner
    // must classify those actual original controls, not the intended formula.
    let rounded = pair(1., 0., -2f64.powi(-80));
    assert_eq!(rounded.first(), pair(1., 0., 0.).first());
    assert_eq!(own(&rounded).kind(), K::TangentDoubleLine);
    let empty = own(&pair(1., 0., -1.5625));
    assert_eq!(empty.kind(), K::Empty);
    assert!(
        empty.vertices().is_empty()
            && empty.edges().is_empty()
            && empty.branches().is_empty()
            && empty.trims().is_empty()
    );
    empty.reverify().unwrap();
    for source in [pair(1., 0., -0.125), pair(9., -1., 0.)] {
        assert!(ss::qualify_quadratic_graph_ss(&source, Default::default()).is_err());
    }
    let mut shifted = graph(1., 0., -0.0625, [[0., 1.]; 2]);
    for row in &mut shifted.control_points {
        for p in row {
            p[2] += 0.75 * p[0] - 0.234375;
        }
    }
    let source = ss::FiberSources::new(&shifted, &graph(0., 0., 0., [[0., 1.]; 2])).unwrap();
    let one = own(&source);
    assert_eq!(one.kind(), K::ParallelLineFactors);
    assert_eq!(
        (
            one.vertices().len(),
            one.edges().len(),
            one.branches().len()
        ),
        (2, 1, 1)
    );
    for t in [0., 0.5, 1.] {
        let station = one.edge_station(0, t, 1024).unwrap();
        contains(station.world[0], 0.375);
        contains(station.world[1], t);
        contains(station.world[2], 0.);
    }
    one.reverify().unwrap();
}

#[test]
fn linear_uv_factors_retain_the_common_curved_world_height_on_both_original_supports() {
    for (a, b, k) in [(4., -1., 0.), (1., 0., -0.0625)] {
        let mut first = graph(a, b, k, [[0., 1.]; 2]);
        let mut second = graph(0., 0., 0., [[0., 1.]; 2]);
        for surface in [&mut first, &mut second] {
            for row in &mut surface.control_points {
                for point in row {
                    point[2] += point[0] * point[1];
                }
            }
        }
        let owner = own(&ss::FiberSources::new(&first, &second).unwrap());
        for edge in 0..owner.edges().len() {
            for t in [0., 0.125, 0.5, 0.875, 1.] {
                let station = owner.edge_station(edge, t, 1024).unwrap();
                let xy = station.normalized_uv.map(|i| i[0] * 0.5 + i[1] * 0.5);
                contains(station.world[2], xy[0] * xy[1]);
            }
        }
        owner.reverify().unwrap();
    }
}
