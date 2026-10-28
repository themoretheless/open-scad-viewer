use super::*;
fn saved(m: &Model, p: &[[f64; 2]]) -> Vec<Segment> {
    p.windows(2)
        .map(|p| Segment {
            curve: crate::line(
                m.faces[0]
                    .surface
                    .evaluate(p[0][0], p[0][1])
                    .unwrap()
                    .point
                    .to_vec(),
                m.faces[0]
                    .surface
                    .evaluate(p[1][0], p[1][1])
                    .unwrap()
                    .point
                    .to_vec(),
            ),
            pcurve: crate::line(p[0].to_vec(), p[1].to_vec()),
        })
        .collect()
}
fn hole(m: &Model, r: [[f64; 2]; 2]) -> Model {
    let [[a, b], [c, d]] = r;
    partition_closed(
        m,
        0,
        &saved(m, &[[a, c], [b, c], [b, d], [a, d], [a, c]]),
        1e-8,
        Default::default(),
    )
    .unwrap()
    .model
}
fn vertex(m: &Model, p: [f64; 2]) -> usize {
    let f = &m.faces[0];
    std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .flat_map(|l| &m.loops[l].coedges)
        .find_map(|c| {
            (c.pcurve.control_points[0] == p.to_vec())
                .then_some(m.edges[c.edge].vertices[usize::from(c.reversed)])
        })
        .unwrap()
}
fn path(m: &Model, a: [f64; 2], b: [f64; 2]) -> BridgePath {
    BridgePath {
        endpoints: [vertex(m, a), vertex(m, b)],
        segments: saved(m, &[a, b]),
    }
}
fn fixture() -> (Model, Vec<BridgePath>) {
    let m = crate::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    let m = hole(
        &hole(
            &hole(&m, [[0.125, 0.25], [0.125, 0.25]]),
            [[0.75, 0.875], [0.125, 0.25]],
        ),
        [[0.4375, 0.5625], [0.75, 0.875]],
    );
    let p = vec![
        path(&m, [0.25, 0.25], [0.75, 0.25]),
        path(&m, [0.875, 0.25], [0.5625, 0.75]),
        path(&m, [0.4375, 0.75], [0.125, 0.25]),
    ];
    (m, p)
}
fn check(m: &Model, p: &[BridgePath], r: &PathGraphPartition) {
    r.model.validate().unwrap();
    assert_eq!(&r.model.edges[..m.edges.len()], m.edges.as_slice());
    assert_eq!(&r.model.vertices[..m.vertices.len()], m.vertices.as_slice());
    for j in 0..p.len() {
        for (i, &e) in r.edges[j].iter().enumerate() {
            assert_eq!(r.model.edges[e].curve, p[j].segments[i].curve);
            let mut owners = Vec::new();
            let mut directions = Vec::new();
            for (f, face) in r.model.faces.iter().enumerate() {
                for l in std::iter::once(face.outer).chain(face.holes.iter().copied()) {
                    for c in &r.model.loops[l].coedges {
                        if c.edge == e {
                            owners.push(f);
                            directions.push(c.reversed);
                        }
                    }
                }
            }
            assert_eq!(owners.len(), 2);
            assert_ne!(directions[0], directions[1]);
            owners.sort();
            let mut expected = r.path_faces[j];
            expected.sort();
            assert_eq!(owners, expected);
        }
    }
    assert_eq!(
        crate::watertight::inspect(&r.model, 100000).unwrap().status,
        crate::watertight::Closure::WithinTolerance
    );
}
#[test]
fn three_different_holes_close_one_bridge_cycle_and_compact_only_owned_loops() {
    let (m, p) = fixture();
    let before = m.clone();
    let r = partition_path_graph(&m, 0, &p, 1e-8, Default::default()).unwrap();
    check(&m, &p, &r);
    assert_eq!(r.faces.len(), 2);
    assert_eq!(r.model.loops.len(), m.loops.len() - 1);
    assert_eq!(r.model.faces[r.faces[0]].holes.len(), 1);
    for i in 1..m.faces.len() {
        assert_eq!(r.model.1.faces[i], m.1.faces[i]);
        let old = &m.faces[i];
        for wire in std::iter::once(old.outer).chain(old.holes.iter().copied()) {
            let index = r
                .model
                .1
                .loops
                .iter()
                .position(|id| *id == m.1.loops[wire])
                .unwrap();
            assert_eq!(r.model.loops[index], m.loops[wire]);
        }
    }
    assert_eq!(m, before);
}
#[test]
fn two_graph_cycles_create_two_caps_and_keep_distinct_path_owner_pairs() {
    let m = crate::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    let m = hole(
        &hole(
            &hole(&m, [[0.125, 0.25], [0.375, 0.625]]),
            [[0.4375, 0.5625], [0.375, 0.625]],
        ),
        [[0.75, 0.875], [0.375, 0.625]],
    );
    let p = vec![
        path(&m, [0.25, 0.375], [0.4375, 0.375]),
        path(&m, [0.25, 0.625], [0.4375, 0.625]),
        path(&m, [0.5625, 0.375], [0.75, 0.375]),
        path(&m, [0.5625, 0.625], [0.75, 0.625]),
    ];
    let r = partition_path_graph(&m, 0, &p, 1e-8, Default::default()).unwrap();
    check(&m, &p, &r);
    assert_eq!(r.faces.len(), 3);
    let mut a = r.path_faces[0];
    a.sort();
    let mut b = r.path_faces[2];
    b.sort();
    assert_ne!(a, b);
}
#[test]
fn ordinary_outer_cut_and_reversed_cycle_paths_use_the_same_closed_walk_engine() {
    let m = crate::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    let p = vec![path(&m, [0., 0.], [1., 1.])];
    let r = partition_path_graph(&m, 0, &p, 1e-8, Default::default()).unwrap();
    check(&m, &p, &r);
    assert_eq!(r.faces.len(), 2);
    let (m, mut p) = fixture();
    p[1] = BridgePath {
        endpoints: [p[1].endpoints[1], p[1].endpoints[0]],
        segments: reverse(&p[1].segments).unwrap(),
    };
    p.reverse();
    check(
        &m,
        &p,
        &partition_path_graph(&m, 0, &p, 1e-8, Default::default()).unwrap(),
    );
}
#[test]
fn dangling_bridge_trees_crossed_paths_and_exhaustion_do_not_publish_slits() {
    let (m, p) = fixture();
    let before = m.clone();
    assert!(partition_path_graph(&m, 0, &p[..1], 1e-8, Default::default()).is_err());
    assert!(partition_path_graph(&m, 0, &p[..2], 1e-8, Default::default()).is_err());
    for l in [
        ClosedCutLimits {
            cells: 1,
            ..Default::default()
        },
        ClosedCutLimits {
            pairs: 1,
            ..Default::default()
        },
    ] {
        assert!(partition_path_graph(&m, 0, &p, 1e-8, l).is_err());
    }
    let m2 = crate::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    let crossed = vec![path(&m2, [0., 0.], [1., 1.]), path(&m2, [1., 0.], [0., 1.])];
    assert!(partition_path_graph(&m2, 0, &crossed, 1e-8, Default::default()).is_err());
    assert_eq!(m, before);
}
#[test]
fn step_round_trip_exports_the_actual_compacted_graph_result() {
    let (m, p) = fixture();
    let r = partition_path_graph(&m, 0, &p, 1e-8, Default::default()).unwrap();
    let (text, out, _) = crate::export_step_v9(&r.model).unwrap();
    assert!(out.complete);
    let (restored, import, _) = crate::import_step_v9(&text).unwrap();
    assert!(import.complete);
    assert_eq!(restored.faces.len(), r.model.faces.len());
    assert_eq!(restored.edges.len(), r.model.edges.len());
    assert_eq!(restored.loops.len(), r.model.loops.len());
    assert_eq!(
        crate::watertight::inspect(&restored, 100000)
            .unwrap()
            .status,
        crate::watertight::Closure::WithinTolerance
    );
}

#[test]
fn an_unconnected_hole_is_deferred_until_the_surrounding_graph_hole_is_assigned() {
    let m = crate::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    let m = hole(&m, [[0.4375, 0.5625], [0.4375, 0.5625]]);
    let original_hole = m.faces[0].holes[0];
    let m = hole(
        &hole(
            &hole(&m, [[0.125, 0.25], [0.125, 0.25]]),
            [[0.75, 0.875], [0.125, 0.25]],
        ),
        [[0.4375, 0.5625], [0.75, 0.875]],
    );
    let id = m.1.loops[original_hole];
    let p = vec![
        path(&m, [0.25, 0.25], [0.75, 0.25]),
        path(&m, [0.875, 0.25], [0.5625, 0.75]),
        path(&m, [0.4375, 0.75], [0.125, 0.25]),
    ];
    let r = partition_path_graph(&m, 0, &p, 1e-8, Default::default()).unwrap();
    check(&m, &p, &r);
    let current = r.model.1.loops.iter().position(|&l| l == id).unwrap();
    assert!(r.model.faces[r.faces[1]].holes.contains(&current));
    assert!(!r.model.faces[r.faces[0]].holes.contains(&current));
}
#[test]
fn rounded_multispan_uv_reversal_cannot_authorize_exact_boundary_cancellation() {
    let m = crate::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    let uv = vec![vec![0., 0.], vec![0.3875, 0.3875], vec![1., 1.]];
    let pc = Curve {
        degree: 1,
        knots: vec![-3., -3., 0.1, 5., 5.],
        control_points: uv.clone(),
        weights: vec![1.; 3],
        periodic: false,
    };
    let world = Curve {
        degree: 1,
        knots: pc.knots.clone(),
        control_points: uv
            .iter()
            .map(|p| {
                m.faces[0]
                    .surface
                    .evaluate(p[0], p[1])
                    .unwrap()
                    .point
                    .to_vec()
            })
            .collect(),
        weights: pc.weights.clone(),
        periodic: false,
    };
    let p = vec![BridgePath {
        endpoints: [vertex(&m, [0., 0.]), vertex(&m, [1., 1.])],
        segments: vec![Segment {
            curve: world,
            pcurve: pc,
        }],
    }];
    let error = partition_path_graph(&m, 0, &p, 1e-8, Default::default())
        .err()
        .unwrap();
    assert!(error.message.contains("exact saved UV reversal"));
}
