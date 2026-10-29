use super::*;
use crate::ss_face_membership::tests::{curves, model};
const STRIP: [[f64; 2]; 2] = [[0.125, 0.875], [0.4375, 0.5625]];
pub(super) fn request(
    input: &QualifiedCutPartitions<'_, '_, '_>,
    upper: bool,
) -> OpenSSPathRequest {
    let node = |right: bool| {
        input
            .source
            .nodes
            .iter()
            .position(|n| (n.position[0] > 0.5) == right && (n.position[1] > 0.5) == upper)
            .unwrap()
    };
    let steps = if upper { [0, 1] } else { [2, 3] };
    OpenSSPathRequest {
        side: 0,
        start_node: node(upper),
        end_node: node(!upper),
        steps: steps
            .into_iter()
            .map(|q| OpenPathStep {
                original_edge: input
                    .source
                    .roots
                    .original
                    .edges()
                    .iter()
                    .position(|e| e.geometry == ss::OriginalGraphSSEdgeGeometry::EllipseQuarter(q))
                    .unwrap(),
                reversed: false,
            })
            .collect(),
    }
}
pub(super) fn combined(a: &Model, b: &Model) -> Model {
    let mut m = a.clone();
    let vo = m.vertices.len();
    let eo = m.edges.len();
    let lo = m.loops.len();
    m.vertices.extend(b.vertices.clone());
    m.edges.extend(b.edges.iter().cloned().map(|mut e| {
        for v in &mut e.vertices {
            *v += vo;
        }
        e
    }));
    m.loops.extend(b.loops.iter().cloned().map(|mut l| {
        for c in &mut l.coedges {
            c.edge += eo;
        }
        l
    }));
    let mut f = b.faces[0].clone();
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
    m
}
#[test]
fn original_four_root_network_builds_both_caps_on_both_operands() {
    let c = curves(0.0625);
    let m = [
        model(c.original().source().first(), Some(STRIP)),
        model(c.original().source().second(), Some(STRIP)),
    ];
    let before = m.clone();
    let roots = qualify_coedge_roots(
        c.original(),
        [(&m[0], 0), (&m[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    assert_eq!(reps.nodes().len(), 4);
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let top = qualify_open_ss_path(&input, request(&input, true), Default::default()).unwrap();
    let bottom = qualify_open_ss_path(&input, request(&input, false), Default::default()).unwrap();
    let network = partition_open_ss_network(&[&top, &bottom], Default::default()).unwrap();
    assert_eq!(network.faces()[0].len(), 3);
    assert_eq!(network.faces()[1].len(), 3);
    assert_eq!(network.cuts().len(), 2);
    for side in 0..2 {
        assert_eq!(network.models()[side].faces.len(), 3);
        for (path, cut) in [&top, &bottom].into_iter().zip(network.cuts()) {
            for (i, &e) in cut.sides[side].edges.iter().enumerate() {
                assert_eq!(
                    network.models()[side].edges[e].curve,
                    path.segments()[i].curve
                );
            }
        }
    }
    network.reverify().unwrap();
    let reversed = partition_open_ss_network(&[&bottom, &top], Default::default()).unwrap();
    reversed.reverify().unwrap();
    assert_eq!(reversed.models()[0].faces.len(), 3);
    let single = partition_open_ss_network(&[&top], Default::default()).unwrap();
    let pair = partition_open_ss_pair(&top, Default::default()).unwrap();
    assert_eq!(single.models(), pair.models());
    assert_eq!(m, before);
}
#[test]
fn aliased_models_accumulate_all_four_native_face_cuts() {
    let c = curves(0.0625);
    let m = combined(
        &model(c.original().source().first(), Some(STRIP)),
        &model(c.original().source().second(), Some(STRIP)),
    );
    let before = m.clone();
    let roots =
        qualify_coedge_roots(c.original(), [(&m, 0), (&m, 1)], 1e-8, Default::default()).unwrap();
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let top = qualify_open_ss_path(&input, request(&input, true), Default::default()).unwrap();
    let bottom = qualify_open_ss_path(&input, request(&input, false), Default::default()).unwrap();
    let result = partition_open_ss_network(&[&top, &bottom], Default::default()).unwrap();
    assert_eq!(result.models().len(), 1);
    assert_eq!(result.models()[0].faces.len(), 6);
    assert_eq!(result.models()[0].shells[0].faces.len(), 3);
    assert_eq!(result.models()[0].shells[1].faces.len(), 3);
    result.reverify().unwrap();
    assert_eq!(m, before);
}
#[test]
fn duplicate_foreign_owner_and_shared_work_refuse_without_partial_models() {
    let c = curves(0.0625);
    let m = [
        model(c.original().source().first(), Some(STRIP)),
        model(c.original().source().second(), Some(STRIP)),
    ];
    let roots = qualify_coedge_roots(
        c.original(),
        [(&m[0], 0), (&m[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let other = partition_cut_representatives(&reps, Default::default()).unwrap();
    let top = qualify_open_ss_path(&input, request(&input, true), Default::default()).unwrap();
    let bottom = qualify_open_ss_path(&input, request(&input, false), Default::default()).unwrap();
    let foreign = qualify_open_ss_path(&other, request(&other, false), Default::default()).unwrap();
    assert!(partition_open_ss_network(&[&top, &top], Default::default()).is_err());
    assert!(partition_open_ss_network(&[&top, &foreign], Default::default()).is_err());
    assert!(partition_open_ss_network(&[], Default::default()).is_err());
    let result = partition_open_ss_network(&[&top, &bottom], Default::default()).unwrap();
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
            operations: result.operations() - 1,
            ..Default::default()
        },
    ] {
        assert!(partition_open_ss_network(&[&top, &bottom], limits).is_err());
    }
    assert_eq!(input.models()[0].faces.len(), 1);
    assert_eq!(input.models()[1].faces.len(), 1);
    result.reverify().unwrap();
}

#[test]
fn crossing_source_branches_share_the_original_junction_on_both_actual_operands() {
    use crate::ss_face_membership::tests::surface;
    let mut a = surface(false, 0.);
    let b = surface(false, 0.);
    for row in &mut a.control_points {
        for p in row {
            p[2] = (p[0] - 0.5) * (p[1] - 0.5);
        }
    }
    let sources = ss::FiberSources::new(&a, &b).unwrap();
    let original = ss::qualify_quadratic_graph_ss(&sources, Default::default()).unwrap();
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
        model(c.original().source().first(), None),
        model(c.original().source().second(), None),
    ];
    let roots = qualify_coedge_roots(
        c.original(),
        [(&m[0], 0), (&m[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let mut paths = Vec::new();
    for branch in original.branches() {
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
        paths.push(
            qualify_open_ss_path(
                &input,
                OpenSSPathRequest {
                    side: 0,
                    start_node: node(ends[0]),
                    end_node: node(ends[1]),
                    steps,
                },
                Default::default(),
            )
            .unwrap(),
        );
    }
    assert_eq!(paths.len(), 2);
    for p in &paths {
        partition_open_ss_pair(p, Default::default()).unwrap();
    }
    let result = partition_open_ss_network(&[&paths[0], &paths[1]], Default::default()).unwrap();
    assert_eq!(result.cuts().len(), 3);
    assert_eq!(result.ports().len(), 1);
    for side in 0..2 {
        assert_eq!(result.models()[side].faces.len(), 4);
        let vertex = result.ports()[0].vertices[side];
        let edges: Vec<_> = result
            .cuts()
            .iter()
            .flat_map(|c| c.sides[side].edges.iter().copied())
            .collect();
        assert_eq!(edges.len(), 4);
        assert_eq!(
            edges
                .iter()
                .filter(|&&e| result.models()[side].edges[e].vertices.contains(&vertex))
                .count(),
            4
        );
    }
    result.reverify().unwrap();
    let mut result = result;
    let ports = result.ports.clone();
    result.ports[0].vertices[0] = usize::MAX;
    assert!(result.reverify().is_err());
    result.ports = ports;
    result.reverify().unwrap();
    let mut reversed_paths = Vec::new();
    for path in &paths {
        let mut r = path.request().clone();
        std::mem::swap(&mut r.start_node, &mut r.end_node);
        r.steps.reverse();
        for step in &mut r.steps {
            step.reversed = !step.reversed;
        }
        reversed_paths.push(qualify_open_ss_path(&input, r, Default::default()).unwrap());
    }
    partition_open_ss_network(
        &[&reversed_paths[0], &reversed_paths[1]],
        Default::default(),
    )
    .unwrap()
    .reverify()
    .unwrap();
    partition_open_ss_network(&[&paths[1], &paths[0]], Default::default())
        .unwrap()
        .reverify()
        .unwrap();
    let alias = combined(&m[0], &m[1]);
    let alias_roots = qualify_coedge_roots(
        c.original(),
        [(&alias, 0), (&alias, 1)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    let alias_reps = qualify_cut_representatives(&alias_roots, Default::default()).unwrap();
    let alias_input = partition_cut_representatives(&alias_reps, Default::default()).unwrap();
    let mut alias_paths = Vec::new();
    for path in &paths {
        let mut r = path.request().clone();
        let node = |old: usize| {
            let v = input.source.nodes[old].original_vertex.unwrap();
            alias_reps
                .nodes()
                .iter()
                .position(|n| n.original_vertex == Some(v))
                .unwrap()
        };
        r.start_node = node(r.start_node);
        r.end_node = node(r.end_node);
        alias_paths.push(qualify_open_ss_path(&alias_input, r, Default::default()).unwrap());
    }
    let aliased =
        partition_open_ss_network(&[&alias_paths[0], &alias_paths[1]], Default::default()).unwrap();
    assert_eq!(aliased.models().len(), 1);
    assert_eq!(aliased.models()[0].faces.len(), 8);
    assert_eq!(aliased.models()[0].shells[0].faces.len(), 4);
    assert_eq!(aliased.models()[0].shells[1].faces.len(), 4);
    aliased.reverify().unwrap();
    assert_eq!(input.models()[0].faces.len(), 1);
    assert_eq!(input.models()[1].faces.len(), 1);
}
