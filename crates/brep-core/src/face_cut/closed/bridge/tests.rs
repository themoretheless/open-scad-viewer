use super::*;
fn saved(m: &Model, points: &[[f64; 2]]) -> Vec<Segment> {
    let s = &m.faces[0].surface;
    points
        .windows(2)
        .map(|p| Segment {
            curve: crate::line(
                s.evaluate(p[0][0], p[0][1]).unwrap().point.to_vec(),
                s.evaluate(p[1][0], p[1][1]).unwrap().point.to_vec(),
            ),
            pcurve: crate::line(p[0].to_vec(), p[1].to_vec()),
        })
        .collect()
}
fn hole(m: &Model, rect: [[f64; 2]; 2]) -> Model {
    let [[a, b], [c, d]] = rect;
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
fn vertex(m: &Model, uv: [f64; 2]) -> usize {
    let f = &m.faces[0];
    std::iter::once(f.outer)
        .chain(f.holes.iter().copied())
        .flat_map(|w| m.loops[w].coedges.iter())
        .find_map(|c| {
            (c.pcurve.control_points[0] == uv.to_vec())
                .then_some(m.edges[c.edge].vertices[usize::from(c.reversed)])
        })
        .unwrap()
}
fn bridge(m: &Model, points: &[[f64; 2]]) -> BridgePath {
    BridgePath {
        endpoints: [vertex(m, points[0]), vertex(m, *points.last().unwrap())],
        segments: saved(m, points),
    }
}
fn fixture(two_holes: bool) -> (Model, [BridgePath; 2]) {
    let m = crate::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    if two_holes {
        let m = hole(
            &hole(&m, [[0.125, 0.375], [0.25, 0.75]]),
            [[0.625, 0.875], [0.25, 0.75]],
        );
        let paths = [
            bridge(&m, &[[0.375, 0.25], [0.625, 0.25]]),
            bridge(&m, &[[0.375, 0.75], [0.625, 0.75]]),
        ];
        (m, paths)
    } else {
        let m = hole(&m, [[0.25, 0.75], [0.25, 0.75]]);
        let paths = [
            bridge(&m, &[[0., 0.], [0.125, 0.0625], [0.25, 0.25]]),
            bridge(&m, &[[1., 0.], [0.875, 0.0625], [0.75, 0.25]]),
        ];
        (m, paths)
    }
}
fn watertight(m: &Model) {
    assert_eq!(
        crate::watertight::inspect(m, 100000).unwrap().status,
        crate::watertight::Closure::WithinTolerance
    );
}
fn check(m: &Model, paths: &[BridgePath; 2], r: &BridgePartition) {
    r.model.validate().unwrap();
    assert_eq!(r.model.faces.len(), m.faces.len() + 1);
    assert_eq!(r.model.loops.len(), m.loops.len());
    assert_eq!(
        r.model.edges.len(),
        m.edges.len() + paths.iter().map(|p| p.segments.len()).sum::<usize>()
    );
    assert!(r.source_uv_reference_exact);
    assert_eq!(&r.model.edges[..m.edges.len()], m.edges.as_slice());
    assert_eq!(&r.model.vertices[..m.vertices.len()], m.vertices.as_slice());
    for j in 0..2 {
        assert_eq!(r.vertices[j][0], paths[j].endpoints[0]);
        assert_eq!(*r.vertices[j].last().unwrap(), paths[j].endpoints[1]);
        for (i, &e) in r.edges[j].iter().enumerate() {
            assert_eq!(r.model.edges[e].curve, paths[j].segments[i].curve);
            let uses: Vec<_> = r
                .model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == e)
                .collect();
            assert_eq!(uses.len(), 2);
            assert_ne!(uses[0].reversed, uses[1].reversed);
        }
    }
    watertight(&r.model);
}
#[test]
fn outer_hole_bridges_form_two_actual_sectors_without_a_slit_or_lost_source_edges() {
    let (mut m, p) = fixture(false);
    let before = m.clone();
    let e = m.1.edges[0];
    let v = m.1.vertices[0];
    m.1.local_tolerances_mm.insert(e, 1e-9);
    m.1.local_tolerances_mm.insert(v, 2e-9);
    m.1.sewed_edges.insert(e);
    let source = m.clone();
    let r = partition_bridges(&m, 0, &p, 1e-8, Default::default()).unwrap();
    check(&m, &p, &r);
    assert!(r.model.faces[r.faces[0]].holes.is_empty());
    assert!(r.model.faces[r.faces[1]].holes.is_empty());
    assert_eq!(r.model.vertices.len(), m.vertices.len() + 2);
    assert_eq!(r.model.1.local_tolerances_mm[&e], 1e-9);
    assert_eq!(r.model.1.local_tolerances_mm[&v], 2e-9);
    assert!(r.model.1.sewed_edges.contains(&e));
    assert_eq!(m, source);
    assert_eq!(before.faces.len(), m.faces.len());
}
#[test]
fn hole_hole_bridges_form_a_cap_and_one_merged_hole() {
    let (m, p) = fixture(true);
    let before = m.clone();
    let r = partition_bridges(&m, 0, &p, 1e-8, Default::default()).unwrap();
    check(&m, &p, &r);
    assert_eq!(r.model.faces[r.faces[0]].holes.len(), 1);
    assert!(r.model.faces[r.faces[1]].holes.is_empty());
    assert_eq!(m, before);
}
#[test]
fn independent_input_directions_and_path_order_preserve_both_bridge_definitions() {
    for two in [false, true] {
        let (m, mut p) = fixture(two);
        p[0] = BridgePath {
            endpoints: [p[0].endpoints[1], p[0].endpoints[0]],
            segments: reverse(&p[0].segments).unwrap(),
        };
        p.swap(0, 1);
        let r = partition_bridges(&m, 0, &p, 1e-8, Default::default()).unwrap();
        check(&m, &p, &r);
    }
}
#[test]
fn remaining_holes_have_unique_complete_region_assignments() {
    for (two, small, side) in [
        (false, [[0.4375, 0.5625], [0.0625, 0.125]], 1usize),
        (false, [[0.4375, 0.5625], [0.8125, 0.875]], 0),
        (true, [[0.4375, 0.5625], [0.4375, 0.5625]], 1),
        (true, [[0.0625, 0.09375], [0.4375, 0.5625]], 0),
    ] {
        let (m, p) = fixture(two);
        let m = hole(&m, small);
        let extra = *m.faces[0].holes.last().unwrap();
        let r = partition_bridges(&m, 0, &p, 1e-8, Default::default()).unwrap();
        check(&m, &p, &r);
        assert!(r.model.faces[r.faces[side]].holes.contains(&extra));
        assert!(!r.model.faces[r.faces[1 - side]].holes.contains(&extra));
        assert_eq!(r.model.loops[extra], m.loops[extra]);
    }
}
#[test]
fn crossed_paths_literal_gaps_malformed_curves_and_shared_work_refuse_atomically() {
    let (m, mut p) = fixture(false);
    let before = m.clone();
    for limits in [
        ClosedCutLimits {
            cells: 1,
            ..Default::default()
        },
        ClosedCutLimits {
            pairs: 1,
            ..Default::default()
        },
    ] {
        assert!(partition_bridges(&m, 0, &p, 1e-8, limits).is_err());
    }
    p[0].segments[0].curve.control_points[0][0] += 1e-10;
    assert!(partition_bridges(&m, 0, &p, 1e-8, Default::default()).is_err());
    let cross = [
        bridge(&m, &[[0., 0.], [0.75, 0.25]]),
        bridge(&m, &[[1., 0.], [0.25, 0.25]]),
    ];
    assert!(partition_bridges(&m, 0, &cross, 1e-8, Default::default()).is_err());
    let (_, mut p) = fixture(false);
    p[0].segments[0].curve.control_points.clear();
    assert!(partition_bridges(&m, 0, &p, 1e-8, Default::default()).is_err());
    let lone = [
        bridge(&m, &[[0., 0.], [0.25, 0.25]]),
        bridge(&m, &[[0., 0.], [0.25, 0.25]]),
    ];
    assert!(partition_bridges(&m, 0, &lone, 1e-8, Default::default()).is_err());
    assert_eq!(m, before);
}
fn rational(m: &Model, a: [f64; 2], mid: [f64; 2], b: [f64; 2]) -> BridgePath {
    let points = [a, mid, b];
    let weights = vec![1., 0.875, 1.];
    let pc = Curve {
        degree: 2,
        knots: vec![-3., -3., -3., 5., 5., 5.],
        control_points: points.iter().map(|p| p.to_vec()).collect(),
        weights: weights.clone(),
        periodic: false,
    };
    let world = Curve {
        degree: 2,
        knots: vec![10., 10., 10., 14., 14., 14.],
        control_points: points
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
        weights,
        periodic: false,
    };
    BridgePath {
        endpoints: [vertex(m, a), vertex(m, b)],
        segments: vec![Segment {
            curve: world,
            pcurve: pc,
        }],
    }
}
#[test]
fn edited_rational_bridge_body_round_trips_step_with_both_original_clocks() {
    let (m, _) = fixture(false);
    let p = [
        rational(&m, [0., 0.], [0.125, 0.0625], [0.25, 0.25]),
        rational(&m, [1., 0.], [0.875, 0.0625], [0.75, 0.25]),
    ];
    let r = partition_bridges(&m, 0, &p, 1e-8, Default::default()).unwrap();
    check(&m, &p, &r);
    let (text, export, _) = crate::export_step_v9(&r.model).unwrap();
    assert!(export.complete);
    let (restored, import, _) = crate::import_step_v9(&text).unwrap();
    assert!(import.complete);
    assert_eq!(restored.faces.len(), r.model.faces.len());
    assert_eq!(restored.edges.len(), r.model.edges.len());
    for path in &p {
        let world = &path.segments[0].curve;
        let edge = restored
            .edges
            .iter()
            .position(|e| e.curve == *world || e.curve == world.reverse().unwrap())
            .expect("Actual edited bridge must survive STEP");
        let uses: Vec<_> = restored
            .loops
            .iter()
            .flat_map(|l| &l.coedges)
            .filter(|c| c.edge == edge)
            .collect();
        assert_eq!(uses.len(), 2);
        assert!(uses.iter().all(|c| c.pcurve.domain() == [-3., 5.]));
    }
    watertight(&restored);
}
