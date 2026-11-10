use super::*;
use crate::ss_face_membership::tests::{curves, fixtures, model, surface};
const HOLE: [[f64; 2]; 2] = [[0.5625, 0.875], [0.4375, 0.5625]];
fn roots<'a>(c: &'a ss::QualifiedGraphCurves, m: &'a [Model; 2]) -> QualifiedFacePairRoots<'a> {
    qualify_coedge_roots(
        c.original(),
        [(&m[0], 0), (&m[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap()
}
fn request(input: &QualifiedCutPartitions<'_, '_, '_>, reverse: bool) -> OpenSSPathRequest {
    let upper = input
        .source
        .nodes
        .iter()
        .position(|n| n.position[1] > 0.5)
        .unwrap();
    let lower = input
        .source
        .nodes
        .iter()
        .position(|n| n.position[1] < 0.5)
        .unwrap();
    let original = input.source.roots.original;
    let edge = |q| {
        original
            .edges()
            .iter()
            .position(|e| e.geometry == ss::OriginalGraphSSEdgeGeometry::EllipseQuarter(q))
            .unwrap()
    };
    OpenSSPathRequest {
        side: 0,
        start_node: if reverse { lower } else { upper },
        end_node: if reverse { upper } else { lower },
        steps: if reverse {
            (0..4)
                .rev()
                .map(|q| OpenPathStep {
                    original_edge: edge(q),
                    reversed: true,
                })
                .collect()
        } else {
            (0..4)
                .map(|q| OpenPathStep {
                    original_edge: edge(q),
                    reversed: false,
                })
                .collect()
        },
    }
}
#[test]
fn original_ellipse_roots_author_a_real_same_hole_face_contour() {
    let c = curves(0.0625);
    let m = fixtures(&c, Some(HOLE));
    let before = m.clone();
    let roots = roots(&c, &m);
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let path = qualify_open_ss_path(&input, request(&input, false), Default::default()).unwrap();
    assert_eq!(path.segments().len(), 4);
    for guard in path.guards() {
        assert!(guard.world_error_upper <= path.world_tolerance());
        assert!(
            guard
                .uv_error_upper
                .iter()
                .all(|&v| v <= path.uv_tolerance())
        );
    }
    let result = partition_open_ss_path(&path, Default::default()).unwrap();
    assert_eq!(result.models()[0].faces.len(), 2);
    assert_eq!(result.edges().len(), 4);
    assert_eq!(result.models()[1], m[1]);
    for (i, &e) in result.edges().iter().enumerate() {
        assert_eq!(result.models()[0].edges[e].curve, path.segments()[i].curve);
    }
    assert_eq!(
        *result.vertices().first().unwrap(),
        path.endpoints()[0].unwrap()
    );
    assert_eq!(
        *result.vertices().last().unwrap(),
        path.endpoints()[1].unwrap()
    );
    path.reverify().unwrap();
    result.reverify().unwrap();
    assert_eq!(m, before);
}
#[test]
fn reversed_source_branch_has_the_same_owned_endpoint_positions_and_valid_sectors() {
    let c = curves(0.0625);
    let m = fixtures(&c, Some(HOLE));
    let roots = roots(&c, &m);
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let path = qualify_open_ss_path(&input, request(&input, true), Default::default()).unwrap();
    let result = partition_open_ss_path(&path, Default::default()).unwrap();
    assert!(path.segments().iter().all(|s| s.reversed));
    assert_eq!(result.models()[0].faces.len(), 2);
    result.reverify().unwrap();
}
#[test]
fn path_through_original_hole_and_unowned_or_disconnected_routes_refuse() {
    let c = curves(0.0625);
    let m = fixtures(&c, Some(HOLE));
    let roots = roots(&c, &m);
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let mut r = request(&input, false);
    let original = r.clone();
    r.steps = vec![
        OpenPathStep {
            original_edge: original.steps[0].original_edge,
            reversed: true,
        },
        OpenPathStep {
            original_edge: original.steps[3].original_edge,
            reversed: true,
        },
    ];
    assert!(qualify_open_ss_path(&input, r, Default::default()).is_err());
    let mut r = original.clone();
    r.steps.swap(1, 2);
    assert!(qualify_open_ss_path(&input, r, Default::default()).is_err());
    let mut r = original.clone();
    r.steps[0].original_edge = usize::MAX;
    assert!(qualify_open_ss_path(&input, r, Default::default()).is_err());
    let mut r = original.clone();
    r.side = 1;
    let interior = qualify_open_ss_path(&input, r, Default::default()).unwrap();
    assert!(interior.endpoints().iter().all(Option::is_none));
    assert!(partition_open_ss_path(&interior, Default::default()).is_err());
    let mut r = original;
    r.end_node = r.start_node;
    assert!(qualify_open_ss_path(&input, r, Default::default()).is_err());
}
#[test]
fn original_graph_vertex_ports_support_open_line_branches() {
    let mut a = surface(false, 0.);
    let b = a.clone();
    for row in &mut a.control_points {
        for p in row {
            p[2] = (p[0] - 0.5) * (p[1] - 0.5);
        }
    }
    let source = ss::FiberSources::new(&a, &b).unwrap();
    let original = ss::qualify_quadratic_graph_ss(&source, Default::default()).unwrap();
    assert_eq!(original.kind(), QuadraticGraphSSKind::CrossingLines);
    let c = ss::construct_graph_curves(
        &original,
        ss::GraphCurveTolerances {
            world: 1e-7,
            uv: 1e-9,
        },
        Default::default(),
    )
    .unwrap();
    let m = fixtures(&c, None);
    let roots = roots(&c, &m);
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let branch = &original.branches()[0];
    let ends: Vec<_> = original
        .vertices()
        .iter()
        .enumerate()
        .filter(|(v, _)| {
            branch
                .edges
                .iter()
                .filter(|&&e| original.edges()[e].vertices.contains(v))
                .count()
                == 1
        })
        .map(|(v, _)| v)
        .collect();
    assert_eq!(ends.len(), 2);
    let mut at = ends[0];
    let mut pending = branch.edges.clone();
    let mut steps = Vec::new();
    while !pending.is_empty() {
        let i = pending
            .iter()
            .position(|&e| original.edges()[e].vertices.contains(&at))
            .unwrap();
        let e = pending.remove(i);
        let ports = original.edges()[e].vertices;
        let reversed = ports[0] != at;
        at = ports[usize::from(!reversed)];
        steps.push(OpenPathStep {
            original_edge: e,
            reversed,
        });
    }
    let node = |v| {
        reps.nodes()
            .iter()
            .position(|n| n.original_vertex == Some(v))
            .unwrap()
    };
    let r = OpenSSPathRequest {
        side: 0,
        start_node: node(ends[0]),
        end_node: node(ends[1]),
        steps,
    };
    let path = qualify_open_ss_path(&input, r, Default::default()).unwrap();
    assert!(path.guards().iter().all(|g| g.clock[0] == g.clock[1]));
    let result = partition_open_ss_path(&path, Default::default()).unwrap();
    assert_eq!(result.models()[0].faces.len(), 2);
    result.reverify().unwrap();
}
#[test]
fn original_nonplanar_ss_height_is_retained_in_the_open_cut() {
    let c = curves(0.0625);
    let mut a = c.original().source().first().clone();
    let mut b = c.original().source().second().clone();
    for s in [&mut a, &mut b] {
        for row in &mut s.control_points {
            for p in row {
                p[2] += 0.125 * p[0] * p[1] + 0.125 * p[0] - 0.25 * p[1] + 0.0625;
            }
        }
    }
    let source = ss::FiberSources::new(&a, &b).unwrap();
    let original = ss::qualify_quadratic_graph_ss(&source, Default::default()).unwrap();
    let c = ss::construct_graph_curves(
        &original,
        ss::GraphCurveTolerances {
            world: 1e-7,
            uv: 1e-9,
        },
        Default::default(),
    )
    .unwrap();
    let m = fixtures(&c, Some(HOLE));
    let roots = roots(&c, &m);
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let path = qualify_open_ss_path(&input, request(&input, false), Default::default()).unwrap();
    let result = partition_open_ss_path(&path, Default::default()).unwrap();
    let z: Vec<_> = result
        .vertices()
        .iter()
        .map(|&v| result.models()[0].vertices[v].point[2])
        .collect();
    assert!(
        z.iter().copied().fold(f64::NEG_INFINITY, f64::max)
            - z.iter().copied().fold(f64::INFINITY, f64::min)
            > 0.01
    );
    result.reverify().unwrap();
}
#[test]
fn path_and_face_replays_reject_mutation_and_exhausted_shared_work() {
    let c = curves(0.0625);
    let m = fixtures(&c, Some(HOLE));
    let roots = roots(&c, &m);
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let r = request(&input, false);
    assert!(
        qualify_open_ss_path(
            &input,
            r.clone(),
            OpenSSPathLimits {
                operations: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        qualify_open_ss_path(
            &input,
            r.clone(),
            OpenSSPathLimits {
                cells: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut path = qualify_open_ss_path(&input, r, Default::default()).unwrap();
    let saved = path.segments.clone();
    path.segments[0].curve.control_points[0][0] += 0.01;
    assert!(path.reverify().is_err());
    path.segments = saved;
    assert!(
        partition_open_ss_path(
            &path,
            OpenSSFaceLimits {
                cells: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    assert!(
        partition_open_ss_path(
            &path,
            OpenSSFaceLimits {
                pairs: 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    let result = partition_open_ss_path(&path, Default::default()).unwrap();
    let mut snapshot = result.models()[0].clone();
    snapshot.vertices[result.vertices()[1]].point[2] += 0.01;
    assert_ne!(snapshot, result.models()[0]);
    result.reverify().unwrap();
}

#[test]
fn a_boundary_event_inside_a_requested_route_requires_a_network_split() {
    let c = curves(0.0625);
    let m = fixtures(&c, Some([[0.125, 0.875], [0.4375, 0.5625]]));
    let roots = roots(&c, &m);
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    assert_eq!(reps.nodes().len(), 4);
    let start_node = reps
        .nodes()
        .iter()
        .position(|n| n.position[0] > 0.5 && n.position[1] > 0.5)
        .unwrap();
    let end_node = reps
        .nodes()
        .iter()
        .position(|n| n.position[0] < 0.5 && n.position[1] < 0.5)
        .unwrap();
    let steps = (0..3)
        .map(|q| OpenPathStep {
            original_edge: c
                .original()
                .edges()
                .iter()
                .position(|e| e.geometry == ss::OriginalGraphSSEdgeGeometry::EllipseQuarter(q))
                .unwrap(),
            reversed: false,
        })
        .collect();
    let error = qualify_open_ss_path(
        &input,
        OpenSSPathRequest {
            side: 0,
            start_node,
            end_node,
            steps,
        },
        Default::default(),
    )
    .err()
    .unwrap();
    assert!(error.message.contains("intermediate"));
}
#[test]
fn independent_other_native_frame_retains_both_saved_pcurve_lifts() {
    let c = curves(0.0625);
    let mut second = c.original().source().second().clone();
    second.knots_u = vec![-3., -3., -3., 5., 5., 5.];
    second.knots_v = vec![7., 7., 7., 11., 11., 11.];
    let source = ss::FiberSources::new(c.original().source().first(), &second).unwrap();
    let original = ss::qualify_quadratic_graph_ss(&source, Default::default()).unwrap();
    let c = ss::construct_graph_curves(
        &original,
        ss::GraphCurveTolerances {
            world: 1e-7,
            uv: 1e-9,
        },
        Default::default(),
    )
    .unwrap();
    let m = [
        model(original.source().first(), Some(HOLE)),
        model(&second, None),
    ];
    let roots = qualify_coedge_roots(
        &original,
        [(&m[0], 0), (&m[1], 0)],
        1e-8,
        CoedgeRootLimits {
            clock_tolerance: 1e-10,
            world_tolerance: 1e-9,
            ..Default::default()
        },
    )
    .unwrap();
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let path = qualify_open_ss_path(&input, request(&input, false), Default::default()).unwrap();
    assert!(
        path.segments()
            .iter()
            .flat_map(|s| &s.pcurves[1].control_points)
            .all(|p| p[1] >= 7. && p[1] <= 11.)
    );
    let result = partition_open_ss_path(&path, Default::default()).unwrap();
    assert_eq!(result.models()[1], m[1]);
    result.reverify().unwrap();
    assert_eq!(c.original().source().second(), &second);
}

#[test]
fn literal_common_boundaries_anchor_both_actual_sides_to_the_same_world_path() {
    let c = curves(0.0625);
    let m = [
        model(c.original().source().first(), Some(HOLE)),
        model(c.original().source().second(), Some(HOLE)),
    ];
    let roots = roots(&c, &m);
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    assert_eq!(reps.nodes().len(), 2);
    let r = request(&input, false);
    let first = qualify_open_ss_path(&input, r.clone(), Default::default()).unwrap();
    let mut r = r;
    r.side = 1;
    let second = qualify_open_ss_path(&input, r, Default::default()).unwrap();
    assert!(
        first
            .segments()
            .iter()
            .zip(second.segments())
            .all(|(a, b)| a.curve == b.curve)
    );
    let a = partition_open_ss_path(&first, Default::default()).unwrap();
    let b = partition_open_ss_path(&second, Default::default()).unwrap();
    assert_eq!(a.models()[0].faces.len(), 2);
    assert_eq!(b.models()[1].faces.len(), 2);
    assert_eq!(a.models()[1], input.models[1]);
    assert_eq!(b.models()[0], input.models[0]);
    a.reverify().unwrap();
    b.reverify().unwrap();
    let pair = partition_open_ss_pair(&first, Default::default()).unwrap();
    assert_eq!(pair.models()[0], a.models()[0]);
    assert_eq!(pair.models()[1], b.models()[1]);
    pair.reverify().unwrap();
    assert!(
        partition_open_ss_pair(
            &first,
            OpenSSFaceLimits {
                operations: pair.operations() - 1,
                ..Default::default()
            }
        )
        .is_err()
    );
    let mut snapshot = pair.models().to_vec();
    snapshot[0].vertices[0].point[0] += 1.;
    assert_ne!(snapshot.as_slice(), pair.models());
    pair.reverify().unwrap();
    let reverse = qualify_open_ss_path(&input, request(&input, true), Default::default()).unwrap();
    partition_open_ss_pair(&reverse, Default::default())
        .unwrap()
        .reverify()
        .unwrap();
    for limits in [
        OpenSSFaceLimits {
            cells: 1,
            ..Default::default()
        },
        OpenSSFaceLimits {
            pairs: 1,
            ..Default::default()
        },
        OpenSSFaceLimits {
            operations: 1,
            ..Default::default()
        },
    ] {
        assert!(partition_open_ss_pair(&first, limits).is_err());
    }
    assert_eq!(input.models()[0].faces.len(), 1);
    assert_eq!(input.models()[1].faces.len(), 1);
}
#[test]
fn one_model_keeps_the_other_selected_face_and_shell_unchanged() {
    let c = curves(0.0625);
    let mut m = model(c.original().source().first(), Some(HOLE));
    let other = model(c.original().source().second(), Some(HOLE));
    let vo = m.vertices.len();
    let eo = m.edges.len();
    let lo = m.loops.len();
    m.vertices.extend(other.vertices.clone());
    m.edges.extend(other.edges.iter().cloned().map(|mut e| {
        for v in &mut e.vertices {
            *v += vo;
        }
        e
    }));
    m.loops.extend(other.loops.iter().cloned().map(|mut l| {
        for c in &mut l.coedges {
            c.edge += eo;
        }
        l
    }));
    let mut f = other.faces[0].clone();
    f.outer += lo;
    for h in &mut f.holes {
        *h += lo;
    }
    m.faces.push(f);
    m.shells.push(crate::Shell {
        faces: vec![crate::FaceUse {
            face: 1,
            reversed: false,
        }],
        closed: false,
    });
    m.rebuild_topology_ids();
    m.validate().unwrap();
    let before = m.clone();
    let roots =
        qualify_coedge_roots(c.original(), [(&m, 0), (&m, 1)], 1e-8, Default::default()).unwrap();
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let path = qualify_open_ss_path(&input, request(&input, false), Default::default()).unwrap();
    let result = partition_open_ss_path(&path, Default::default()).unwrap();
    assert_eq!(result.models().len(), 1);
    assert_eq!(result.models()[0].faces.len(), 3);
    assert_eq!(result.models()[0].faces[1], input.models[0].faces[1]);
    assert_eq!(result.models()[0].shells[1], input.models[0].shells[1]);
    for wire in std::iter::once(input.models[0].faces[1].outer)
        .chain(input.models[0].faces[1].holes.iter().copied())
    {
        assert_eq!(result.models()[0].loops[wire], input.models[0].loops[wire]);
    }
    result.reverify().unwrap();
    let pair = partition_open_ss_pair(&path, Default::default()).unwrap();
    assert_eq!(pair.models().len(), 1);
    assert_eq!(pair.models()[0].faces.len(), 4);
    assert_eq!(pair.sides()[0].model, pair.sides()[1].model);
    assert_ne!(pair.sides()[0].edges, pair.sides()[1].edges);
    for &f in &pair.sides()[0].faces {
        assert_eq!(pair.models()[0].faces[f], result.models()[0].faces[f]);
        for w in std::iter::once(result.models()[0].faces[f].outer)
            .chain(result.models()[0].faces[f].holes.iter().copied())
        {
            assert_eq!(pair.models()[0].loops[w], result.models()[0].loops[w]);
        }
    }
    pair.reverify().unwrap();
    assert_eq!(m, before);
}

#[test]
fn a_wrapped_branch_keeps_two_root_bands_on_one_source_edge_separate() {
    let c = curves(0.0625);
    let m = fixtures(&c, Some([[0.625, 0.875], [0.625, 0.875]]));
    let roots = roots(&c, &m);
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    assert_eq!(reps.nodes().len(), 2);
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let start_node = if reps.nodes()[0].position[1] > reps.nodes()[1].position[1] {
        0
    } else {
        1
    };
    let end_node = 1 - start_node;
    assert_eq!(
        reps.nodes()[start_node].original_edge,
        reps.nodes()[end_node].original_edge
    );
    let steps = (0..4)
        .chain([0])
        .map(|q| OpenPathStep {
            original_edge: c
                .original()
                .edges()
                .iter()
                .position(|e| e.geometry == ss::OriginalGraphSSEdgeGeometry::EllipseQuarter(q))
                .unwrap(),
            reversed: false,
        })
        .collect();
    let path = qualify_open_ss_path(
        &input,
        OpenSSPathRequest {
            side: 0,
            start_node,
            end_node,
            steps,
        },
        Default::default(),
    )
    .unwrap();
    assert_eq!(path.segments().len(), 5);
    assert!(path.guards()[0].clock[0] > path.guards()[1].clock[1]);
    let result = partition_open_ss_path(&path, Default::default()).unwrap();
    assert_eq!(result.edges().len(), 5);
    assert_eq!(result.models()[0].faces.len(), 2);
    result.reverify().unwrap();
}

#[test]
fn endpoints_on_different_operands_still_have_a_complete_source_geometry_path() {
    let c = curves(0.0625);
    let shifted = [[0.5625, 0.875], [0.453125, 0.578125]];
    let m = [
        model(c.original().source().first(), Some(HOLE)),
        model(c.original().source().second(), Some(shifted)),
    ];
    let roots = roots(&c, &m);
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    assert_eq!(reps.nodes().len(), 2);
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let path = qualify_open_ss_path(&input, request(&input, false), Default::default()).unwrap();
    assert!(path.endpoints()[0].is_none());
    assert!(path.endpoints()[1].is_some());
    assert_eq!(path.segments().len(), 4);
    path.reverify().unwrap();
    let error = partition_open_ss_path(&path, Default::default())
        .err()
        .unwrap();
    assert!(error.message.contains("network"));
    assert!(partition_open_ss_pair(&path, Default::default()).is_err());
    assert_eq!(input.models()[0].faces.len(), 1);
    assert_eq!(input.models()[1].faces.len(), 1);
}

#[test]
fn exact_elevated_query_identity_routes_and_authors_both_original_ss_faces() {
    let c = curves(0.0625);
    let mut m = [
        model(c.original().source().first(), Some(HOLE)),
        model(c.original().source().second(), Some(HOLE)),
    ];
    super::super::tests::elevated_boundary_clocks(&mut m[1]);
    let before = m.clone();
    let roots = roots(&c, &m);
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    for reverse in [false, true] {
        let first =
            qualify_open_ss_path(&input, request(&input, reverse), Default::default()).unwrap();
        let mut other = request(&input, reverse);
        other.side = 1;
        let second = qualify_open_ss_path(&input, other, Default::default()).unwrap();
        assert!(
            first
                .segments()
                .iter()
                .zip(second.segments())
                .all(|(a, b)| a.curve == b.curve)
        );
        let pair = partition_open_ss_pair(&first, Default::default()).unwrap();
        assert_eq!(pair.models()[0].faces.len(), 2);
        assert_eq!(pair.models()[1].faces.len(), 2);
        for side in 0..2 {
            let independent = partition_open_ss_path(
                if side == 0 { &first } else { &second },
                Default::default(),
            )
            .unwrap();
            assert_eq!(pair.models()[side], independent.models()[side]);
        }
        pair.reverify().unwrap();
    }
    assert_eq!(m, before);
}
