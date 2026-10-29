use super::*;
fn path(model: &Model, points: &[[f64; 2]]) -> Vec<Segment> {
    let s = &model.faces[0].surface;
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
fn closed(model: &Model, points: &[[f64; 2]]) -> ClosedPartition {
    let mut p = points.to_vec();
    p.push(points[0]);
    partition_closed(model, 0, &path(model, &p), 1e-8, Default::default()).unwrap()
}
fn fixture() -> (Model, [usize; 2], Vec<Segment>) {
    let m = crate::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    let first = closed(
        &m,
        &[[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.75]],
    );
    let ends = [first.vertices[0], first.vertices[1]];
    let model = first.model;
    let cut = path(
        &model,
        &[[0.25, 0.25], [0.25, 0.125], [0.75, 0.125], [0.75, 0.25]],
    );
    (model, ends, cut)
}
#[test]
fn same_hole_path_creates_material_cap_and_enlarged_hole_in_a_closed_shell() {
    let (m, ends, p) = fixture();
    let before = m.clone();
    let hole = m.faces[0].holes[0];
    let out = partition_same_hole(&m, 0, ends, &p, 1e-8, Default::default()).unwrap();
    assert_eq!(out.model.faces.len(), m.faces.len() + 1);
    assert_eq!(out.model.edges.len(), m.edges.len() + 3);
    assert_eq!(out.model.vertices.len(), m.vertices.len() + 2);
    assert_eq!(out.model.loops.len(), m.loops.len() + 1);
    assert_eq!(out.model.faces[out.faces[0]].holes, [hole]);
    assert!(out.model.faces[out.faces[1]].holes.is_empty());
    assert!(!out.cap_path_reversed);
    assert!(out.source_uv_reference_exact);
    for &e in &out.edges {
        let uses: Vec<_> = out
            .model
            .loops
            .iter()
            .flat_map(|l| &l.coedges)
            .filter(|c| c.edge == e)
            .collect();
        assert_eq!(uses.len(), 2);
        assert_ne!(uses[0].reversed, uses[1].reversed);
    }
    assert_eq!(
        crate::watertight::inspect(&out.model, 100000)
            .unwrap()
            .status,
        crate::watertight::Closure::WithinTolerance
    );
    assert_eq!(m, before);
}
#[test]
fn reversed_path_keeps_the_same_original_hole_and_infers_cap_orientation() {
    let (m, ends, p) = fixture();
    let p = reverse(&p).unwrap();
    let out = partition_same_hole(&m, 0, [ends[1], ends[0]], &p, 1e-8, Default::default()).unwrap();
    assert!(out.cap_path_reversed);
    assert_eq!(out.vertices[0], ends[1]);
    assert_eq!(*out.vertices.last().unwrap(), ends[0]);
    out.model.validate().unwrap();
}
#[test]
fn another_original_hole_is_assigned_by_complete_region_proofs() {
    for (small, side) in [
        (
            [
                [0.4375, 0.15625],
                [0.5625, 0.15625],
                [0.5625, 0.21875],
                [0.4375, 0.21875],
            ],
            1usize,
        ),
        (
            [
                [0.8125, 0.125],
                [0.875, 0.125],
                [0.875, 0.1875],
                [0.8125, 0.1875],
            ],
            0usize,
        ),
    ] {
        let (m, ends, p) = fixture();
        let added = closed(&m, &small);
        let m = added.model;
        let other_hole = *m.faces[0].holes.last().unwrap();
        let before = m.clone();
        let out = partition_same_hole(&m, 0, ends, &p, 1e-8, Default::default()).unwrap();
        assert!(out.model.faces[out.faces[side]].holes.contains(&other_hole));
        assert!(
            !out.model.faces[out.faces[1 - side]]
                .holes
                .contains(&other_hole)
        );
        assert_eq!(out.model.loops[other_hole], m.loops[other_hole]);
        assert_eq!(
            crate::watertight::inspect(&out.model, 100000)
                .unwrap()
                .status,
            crate::watertight::Closure::WithinTolerance
        );
        assert_eq!(m, before);
    }
}
#[test]
fn paths_inside_hole_crossings_and_sub_tolerance_endpoint_gaps_are_refused() {
    let (m, ends, p) = fixture();
    let before = m.clone();
    for points in [
        vec![[0.25, 0.25], [0.5, 0.5], [0.75, 0.25]],
        vec![[0.25, 0.25], [0.7, 0.125], [0.3, 0.125], [0.75, 0.25]],
    ] {
        assert!(
            partition_same_hole(&m, 0, ends, &path(&m, &points), 1e-8, Default::default()).is_err()
        );
    }
    let mut bad = p;
    bad[0].pcurve.control_points[0][0] += 1e-12;
    assert!(partition_same_hole(&m, 0, ends, &bad, 1e-8, Default::default()).is_err());
    assert_eq!(m, before);
}
#[test]
fn source_policy_and_original_world_curve_definitions_survive_hole_partition() {
    let (mut m, ends, p) = fixture();
    let edge = m.1.edges[0];
    let vertex = m.1.vertices[ends[0]];
    m.1.local_tolerances_mm.insert(edge, 1e-9);
    m.1.local_tolerances_mm.insert(vertex, 2e-9);
    m.1.sewed_edges.insert(edge);
    let touched_edge = m.1.edges[m.loops[m.faces[0].holes[0]].coedges.last().unwrap().edge];
    m.1.local_tolerances_mm.insert(touched_edge, 3e-9);
    m.1.sewed_edges.insert(touched_edge);
    let before = m.clone();
    let out = partition_same_hole(&m, 0, ends, &p, 1e-8, Default::default()).unwrap();
    assert_eq!(out.model.1.local_tolerances_mm[&edge], 1e-9);
    assert_eq!(out.model.1.local_tolerances_mm[&vertex], 2e-9);
    assert!(out.model.1.sewed_edges.contains(&edge));
    assert_eq!(out.model.1.local_tolerances_mm[&touched_edge], 3e-9);
    assert!(out.model.1.sewed_edges.contains(&touched_edge));
    assert_eq!(&out.model.edges[..m.edges.len()], &m.edges[..]);
    assert_eq!(m, before);
}
#[test]
fn rounded_pcurve_reversal_checks_the_original_reference_and_budget() {
    let (m, ends, mut p) = fixture();
    p[0].pcurve = p[0].pcurve.insert(0.1, 1).unwrap();
    let out = partition_same_hole(&m, 0, ends, &p, 1e-8, Default::default()).unwrap();
    assert!(!out.source_uv_reference_exact);
    let exact = ClosedCutLimits {
        cells: out.cells,
        pairs: out.pairs,
    };
    // Adaptive per-audit allocation makes observed work a report, not a
    // promised minimal sufficient budget. Accept only a bounded result or an
    // explicit inconclusive proof, never an unchecked partial construction.
    match partition_same_hole(&m, 0, ends, &p, 1e-8, exact) {
        Ok(r) => {
            assert!(r.cells <= exact.cells);
            assert!(r.pairs <= exact.pairs);
        }
        Err(e) => assert_eq!(e.code, "BREP_FACE_CUT_UNPROVEN"),
    }
    assert!(
        partition_same_hole(&m, 0, ends, &p, 1e-8, ClosedCutLimits { cells: 1, ..exact }).is_err()
    );
    assert!(
        partition_same_hole(&m, 0, ends, &p, 1e-8, ClosedCutLimits { pairs: 1, ..exact }).is_err()
    );
}
#[test]
fn malformed_paths_and_endpoints_on_different_loops_refuse_without_panics() {
    let (m, ends, mut p) = fixture();
    p[1].curve.control_points.clear();
    assert!(partition_same_hole(&m, 0, ends, &p, 1e-8, Default::default()).is_err());
    let (_, _, p) = fixture();
    let outer_vertex = m.edges[m.loops[m.faces[0].outer].coedges[0].edge].vertices[0];
    assert!(
        partition_same_hole(&m, 0, [ends[0], outer_vertex], &p, 1e-8, Default::default()).is_err()
    );
}

#[test]
fn rational_curved_hole_cut_keeps_independent_native_clocks() {
    let (m, ends, _) = fixture();
    let segment = rational_path(&m);
    let world = segment.curve.clone();
    let out = partition_same_hole(&m, 0, ends, &[segment], 1e-8, Default::default()).unwrap();
    assert_eq!(out.edges.len(), 1);
    assert_eq!(out.vertices, ends);
    assert_eq!(out.model.edges[out.edges[0]].curve, world);
    assert!(out.source_uv_reference_exact);
    assert_eq!(
        crate::watertight::inspect(&out.model, 100000)
            .unwrap()
            .status,
        crate::watertight::Closure::WithinTolerance
    );
}
#[test]
fn existing_partition_projection_network_and_retention_routes_accept_same_hole() {
    let (m, ends, p) = fixture();
    let before = m.clone();
    let direct = super::super::super::partition(&m, 0, ends, &p, 1e-8, 100000).unwrap();
    assert_eq!(direct.model.faces.len(), m.faces.len() + 1);
    let world: Vec<_> = p.iter().map(|s| s.curve.clone()).collect();
    let projected =
        super::super::super::partition_projected(&m, 0, ends, &world, 1e-8, 100000).unwrap();
    assert_eq!(projected.model.faces.len(), m.faces.len() + 1);
    let owned = [super::super::super::OwnedCut {
        endpoints: ends,
        curves: world,
    }];
    let network =
        super::super::super::partition_projected_network(&m, 0, &owned, 1e-8, 100000, 1000000)
            .unwrap();
    assert_eq!(network.faces.len(), 2);
    let plan = super::super::super::plan_projected_retention(
        &m,
        0,
        &owned,
        &[[0.5, 0.1875]],
        1e-8,
        100000,
        1000000,
        100000,
    )
    .unwrap();
    assert_eq!(plan.retained_faces.len(), 1);
    assert_eq!(plan.discard_faces.len(), 1);
    assert_ne!(plan.retained_faces[0], 0);
    assert_eq!(plan.discard_faces, [0]);
    assert_eq!(m, before);
}

#[test]
fn two_projected_hole_cuts_keep_unique_current_sector_ownership_and_both_caps() {
    let (m, ends, lower) = fixture();
    let before = m.clone();
    let hole = m.faces[0].holes[0];
    let find = |uv: [f64; 2]| {
        m.loops[hole]
            .coedges
            .iter()
            .find(|c| c.pcurve.control_points[0].as_slice() == uv)
            .map(|c| m.edges[c.edge].vertices[usize::from(c.reversed)])
            .unwrap()
    };
    let upper_ends = [find([0.75, 0.75]), find([0.25, 0.75])];
    let upper = path(
        &m,
        &[[0.75, 0.75], [0.75, 0.875], [0.25, 0.875], [0.25, 0.75]],
    );
    let cuts = [
        super::super::super::OwnedCut {
            endpoints: ends,
            curves: lower.into_iter().map(|s| s.curve).collect(),
        },
        super::super::super::OwnedCut {
            endpoints: upper_ends,
            curves: upper.into_iter().map(|s| s.curve).collect(),
        },
    ];
    let out = super::super::super::plan_projected_retention(
        &m,
        0,
        &cuts,
        &[[0.5, 0.1875], [0.5, 0.8125]],
        1e-8,
        100000,
        1000000,
        100000,
    )
    .unwrap();
    assert_eq!(out.network.faces.len(), 3);
    assert_eq!(out.retained_faces.len(), 2);
    assert_eq!(out.discard_faces, [0]);
    assert_eq!(
        crate::watertight::inspect(&out.network.model, 100000)
            .unwrap()
            .status,
        crate::watertight::Closure::WithinTolerance
    );
    assert_eq!(m, before);
}

#[test]
fn original_outer_route_preserves_policy_and_checks_rounded_uv_references() {
    let mut m = crate::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    let ring = &m.loops[m.faces[0].outer].coedges;
    let ends = [
        m.edges[ring[0].edge].vertices[usize::from(ring[0].reversed)],
        m.edges[ring[2].edge].vertices[usize::from(ring[2].reversed)],
    ];
    let edge = m.1.edges[ring[0].edge];
    let vertex = m.1.vertices[ends[0]];
    m.1.local_tolerances_mm.insert(edge, 1e-9);
    m.1.local_tolerances_mm.insert(vertex, 2e-9);
    m.1.sewed_edges.insert(edge);
    let pc = crate::line(vec![0., 0.], vec![1., 1.])
        .insert(0.1, 1)
        .unwrap();
    let mut world = pc.clone();
    world.control_points = pc
        .control_points
        .iter()
        .map(|p| {
            m.faces[0]
                .surface
                .evaluate(p[0], p[1])
                .unwrap()
                .point
                .to_vec()
        })
        .collect();
    let before = m.clone();
    let out = super::super::super::partition(
        &m,
        0,
        ends,
        &[Segment {
            curve: world,
            pcurve: pc,
        }],
        1e-8,
        100000,
    )
    .unwrap();
    assert_eq!(out.model.1.local_tolerances_mm[&edge], 1e-9);
    assert_eq!(out.model.1.local_tolerances_mm[&vertex], 2e-9);
    assert!(out.model.1.sewed_edges.contains(&edge));
    assert_eq!(
        crate::watertight::inspect(&out.model, 100000)
            .unwrap()
            .status,
        crate::watertight::Closure::WithinTolerance
    );
    assert_eq!(m, before);
}

fn rational_path(m: &Model) -> Segment {
    let pc = Curve {
        degree: 2,
        knots: vec![-3., -3., -3., 5., 5., 5.],
        control_points: vec![vec![0.25, 0.25], vec![0.5, 0.], vec![0.75, 0.25]],
        weights: vec![1., 0.75, 1.],
        periodic: false,
    };
    let mut world = pc.clone();
    world.knots = vec![10., 10., 10., 14., 14., 14.];
    world.control_points = pc
        .control_points
        .iter()
        .map(|p| {
            m.faces[0]
                .surface
                .evaluate(p[0], p[1])
                .unwrap()
                .point
                .to_vec()
        })
        .collect();
    Segment {
        curve: world,
        pcurve: pc,
    }
}

#[test]
fn step_round_trip_exports_the_edited_hole_cap_and_original_rational_clocks() {
    let (m, ends, _) = fixture();
    let segment = rational_path(&m);
    let world = segment.curve.clone();
    let edited = partition_same_hole(&m, 0, ends, &[segment], 1e-8, Default::default())
        .unwrap()
        .model;
    let (step, export_certificate, _) = crate::export_step_v9(&edited).unwrap();
    assert!(export_certificate.complete);
    let (restored, import_certificate, _) = crate::import_step_v9(&step).unwrap();
    assert!(import_certificate.complete);
    assert_eq!(restored.faces.len(), edited.faces.len());
    assert_eq!(restored.edges.len(), edited.edges.len());
    assert_eq!(restored.faces.len(), m.faces.len() + 1);
    assert_eq!(restored.edges.len(), m.edges.len() + 1);
    let imported = restored
        .edges
        .iter()
        .enumerate()
        .find(|(_, e)| e.curve == world || e.curve == world.reverse().unwrap())
        .map(|(i, _)| i)
        .expect("Edited rational world curve must survive export/import");
    let uses: Vec<_> = restored
        .loops
        .iter()
        .flat_map(|l| &l.coedges)
        .filter(|c| c.edge == imported)
        .collect();
    assert_eq!(uses.len(), 2);
    assert!(uses.iter().all(|c| c.pcurve.domain() == [-3., 5.]));
    assert_eq!(
        crate::watertight::inspect(&restored, 100000)
            .unwrap()
            .status,
        crate::watertight::Closure::WithinTolerance
    );
}
