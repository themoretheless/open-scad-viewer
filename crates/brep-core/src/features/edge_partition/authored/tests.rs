use super::*;
fn fixture() -> (Model, f64, Vec<PcurveCut>, [f64; 3]) {
    let model = crate::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    let [a, b] = model.edges[0].curve.domain();
    let t = a + (b - a) * 0.25;
    let p = model.edges[0]
        .curve
        .evaluate(t)
        .unwrap()
        .point
        .try_into()
        .unwrap();
    let cuts = model
        .loops
        .iter()
        .enumerate()
        .flat_map(|(wire, l)| {
            l.coedges
                .iter()
                .enumerate()
                .filter(|(_, c)| c.edge == 0)
                .map(move |(coedge, c)| {
                    let [a, b] = c.pcurve.domain();
                    PcurveCut {
                        wire,
                        coedge,
                        parameter: a + (b - a) * if c.reversed { 0.75 } else { 0.25 },
                    }
                })
        })
        .collect();
    (model, t, cuts, p)
}
#[test]
fn canonical_position_updates_both_faces_without_mutating_source() {
    let (model, t, cuts, mut p) = fixture();
    let before = model.clone();
    p[2] += 1e-9;
    let out = partition_with_position(&model, 0, t, &cuts, p, 1e-8, 10000).unwrap();
    assert_eq!(model, before);
    assert_eq!(out.model.vertices[out.vertex].point, p);
    for child in out.child_edges {
        assert_eq!(
            out.model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == child)
                .count(),
            2
        );
        let e = &out.model.edges[child];
        let pole = if e.vertices[0] == out.vertex {
            0
        } else {
            e.curve.control_points.len() - 1
        };
        assert_eq!(e.curve.control_points[pole], p);
    }
    out.model.validate().unwrap();
}
#[test]
fn canonical_position_refuses_outside_point_budget_and_tolerance_increase() {
    let (model, t, cuts, p) = fixture();
    let before = model.clone();
    let mut wrong = p;
    wrong[2] += 0.01;
    assert!(partition_with_position(&model, 0, t, &cuts, wrong, 1e-8, 10000).is_err());
    assert!(partition_with_position(&model, 0, t, &cuts, p, 1e-8, 1).is_err());
    assert!(
        partition_with_position(&model, 0, t, &cuts, p, model.tolerance_mm * 2., 10000).is_err()
    );
    assert_eq!(model, before);
}
#[test]
fn canonical_partition_preserves_local_policy_and_complete_windows() {
    let (mut model, t, cuts, p) = fixture();
    let source_id = model.1.edges[0];
    let unrelated = model.1.edges[1];
    let vertex = model.1.vertices[0];
    model.1.local_tolerances_mm.insert(source_id, 1e-9);
    model.1.local_tolerances_mm.insert(unrelated, 2e-9);
    model.1.local_tolerances_mm.insert(vertex, 3e-9);
    model.1.sewed_edges.insert(source_id);
    model.1.sewed_edges.insert(unrelated);
    for cut in &cuts {
        let c = model.loops[cut.wire].coedges[cut.coedge].clone();
        let window = brep_topology::CoedgeTrim {
            curve_parameter: model.edges[0].curve.domain(),
            pcurve_parameter: c.pcurve.domain(),
            periodic_lift: [[0; 2]; 2],
        };
        model.loops[cut.wire].coedges[cut.coedge] = c.with_trim(window).unwrap();
    }
    assert!(partition_with_position(&model, 0, t, &cuts, p, 2e-9, 10000).is_err());
    let out = partition_with_position(&model, 0, t, &cuts, p, 1e-9, 10000).unwrap();
    assert_eq!(out.model.1.local_tolerances_mm[&unrelated], 2e-9);
    assert_eq!(out.model.1.local_tolerances_mm[&vertex], 3e-9);
    assert!(out.model.1.sewed_edges.contains(&unrelated));
    for child in out.child_edges {
        let id = out.model.1.edges[child];
        assert_eq!(out.model.1.local_tolerances_mm[&id], 1e-9);
        assert!(out.model.1.sewed_edges.contains(&id));
        for c in out
            .model
            .loops
            .iter()
            .flat_map(|l| &l.coedges)
            .filter(|c| c.edge == child)
        {
            let window = c.trim().unwrap();
            assert_eq!(
                window.curve_parameter,
                out.model.edges[child].curve.domain()
            );
            assert_eq!(window.pcurve_parameter, c.pcurve.domain());
        }
    }
    out.model.validate().unwrap();
}
