use super::super::tests::combined;
use super::*;
use crate::ss_face_membership::tests::{curves, polygon_regions};
fn fixtures(c: &ss::QualifiedGraphCurves) -> [Model; 2] {
    let outer = vec![[0., 0.], [1., 0.], [1., 1.], [0., 1.]];
    let holes = vec![
        vec![
            [0.6875, 0.4375],
            [0.875, 0.4375],
            [0.875, 0.5625],
            [0.6875, 0.5625],
        ],
        vec![
            [0.25, 0.625],
            [0.4375, 0.625],
            [0.4375, 0.8125],
            [0.25, 0.8125],
        ],
        vec![
            [0.25, 0.1875],
            [0.4375, 0.1875],
            [0.4375, 0.375],
            [0.25, 0.375],
        ],
    ];
    [
        polygon_regions(c.original().source().first(), outer.clone(), holes.clone()),
        polygon_regions(c.original().source().second(), outer, holes),
    ]
}
fn requests(input: &QualifiedCutPartitions<'_, '_, '_>) -> Vec<OpenSSPathRequest> {
    let node = |right: bool, upper: bool, near: bool| {
        input
            .source
            .nodes
            .iter()
            .position(|n| {
                if right {
                    n.position[0] > 0.6 && (n.position[1] > 0.5) == upper
                } else {
                    n.position[0] < 0.6
                        && (n.position[1] > 0.5) == upper
                        && (n.position[0] > 0.4) == near
                }
            })
            .unwrap()
    };
    let pairs = [
        (node(true, true, false), node(false, true, true), [0, 1]),
        (node(false, true, false), node(false, false, false), [1, 2]),
        (node(false, false, true), node(true, false, false), [2, 3]),
    ];
    pairs
        .into_iter()
        .map(|(start_node, end_node, q)| OpenSSPathRequest {
            side: 0,
            start_node,
            end_node,
            steps: q
                .into_iter()
                .map(|q| OpenPathStep {
                    original_edge: input
                        .source
                        .roots
                        .original
                        .edges()
                        .iter()
                        .position(|e| {
                            e.geometry == ss::OriginalGraphSSEdgeGeometry::EllipseQuarter(q)
                        })
                        .unwrap(),
                    reversed: false,
                })
                .collect(),
        })
        .collect()
}
fn verify(r: &QualifiedOpenSSNetwork<'_, '_, '_, '_, '_>) {
    assert_eq!(r.faces()[0].len(), 2);
    assert_eq!(r.faces()[1].len(), 2);
    assert_eq!(r.cuts().len(), 3);
    assert_eq!(r.ports().len(), 3);
    for cut in r.cuts() {
        assert_eq!(cut.bridge_groups, [Some(0); 2]);
        for side in 0..2 {
            let o = &cut.sides[side];
            let m = &r.models()[o.model];
            for &e in &o.edges {
                let mut actual = Vec::new();
                for (i, f) in m.faces.iter().enumerate() {
                    for l in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
                        if m.loops[l].coedges.iter().any(|c| c.edge == e) {
                            actual.push(i);
                        }
                    }
                }
                actual.sort();
                let mut expected = o.faces;
                expected.sort();
                assert_eq!(actual, expected);
            }
        }
    }
    r.reverify().unwrap();
}
#[test]
fn three_source_paths_join_three_distinct_hole_pairs_as_one_actual_cycle() {
    let c = curves(0.0625);
    let m = fixtures(&c);
    let before = m.clone();
    let roots = qualify_coedge_roots(
        c.original(),
        [(&m[0], 0), (&m[1], 0)],
        1e-8,
        Default::default(),
    )
    .unwrap();
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    assert_eq!(reps.nodes().len(), 6);
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let paths: Vec<_> = requests(&input)
        .into_iter()
        .map(|r| qualify_open_ss_path(&input, r, Default::default()).unwrap())
        .collect();
    let refs: Vec<_> = paths.iter().collect();
    let r = partition_open_ss_network(&refs, Default::default()).unwrap();
    verify(&r);
    for side in 0..2 {
        assert_eq!(r.models()[side].faces[r.faces()[side][0]].holes.len(), 1);
    }
    let reversed: Vec<_> = refs.iter().rev().copied().collect();
    verify(&partition_open_ss_network(&reversed, Default::default()).unwrap());
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
            operations: r.operations() - 1,
            ..Default::default()
        },
    ] {
        assert!(partition_open_ss_network(&refs, limits).is_err());
    }
    assert_eq!(m, before);
}
#[test]
fn one_model_retains_both_general_bridge_graphs_and_edited_step_topology() {
    let c = curves(0.0625);
    let pair = fixtures(&c);
    let m = combined(&pair[0], &pair[1]);
    let roots =
        qualify_coedge_roots(c.original(), [(&m, 0), (&m, 1)], 1e-8, Default::default()).unwrap();
    let reps = qualify_cut_representatives(&roots, Default::default()).unwrap();
    let input = partition_cut_representatives(&reps, Default::default()).unwrap();
    let paths: Vec<_> = requests(&input)
        .into_iter()
        .map(|r| qualify_open_ss_path(&input, r, Default::default()).unwrap())
        .collect();
    let refs: Vec<_> = paths.iter().collect();
    let r = partition_open_ss_network(&refs, Default::default()).unwrap();
    verify(&r);
    assert_eq!(r.models().len(), 1);
    assert_eq!(r.models()[0].faces.len(), 4);
    assert_eq!(r.models()[0].shells[0].faces.len(), 2);
    assert_eq!(r.models()[0].shells[1].faces.len(), 2);
    let (text, out, _) = crate::export_step_v9(&r.models()[0]).unwrap();
    assert!(out.complete);
    let (restored, import, _) = crate::import_step_v9(&text).unwrap();
    assert!(import.complete);
    assert_eq!(restored.faces.len(), 4);
    assert_eq!(restored.edges.len(), r.models()[0].edges.len());
    assert_eq!(restored.shells.len(), 2);
}
