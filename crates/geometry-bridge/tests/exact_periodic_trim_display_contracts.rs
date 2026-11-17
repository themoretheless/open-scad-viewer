use geometry_bridge::brep::tessellate_exact_periodic_trim_sheet;
use nurbs_core::{
    curve::Curve,
    surface::{Surface, extrude},
};
use std::collections::{BTreeMap, BTreeSet};

fn support() -> Surface {
    extrude(
        &Curve {
            degree: 1,
            knots: vec![-1., 0., 1., 2., 3., 4., 5.],
            control_points: vec![
                vec![0., 0., 0.],
                vec![1., 0., 0.],
                vec![1., 1., 0.],
                vec![0., 1., 0.],
                vec![0., 0., 0.],
            ],
            weights: vec![1.; 5],
            periodic: true,
        },
        [0., 0., 2.],
    )
    .unwrap()
}

#[test]
fn periodic_trim_display_keeps_two_mesh_boundary_loops_and_face_identity() {
    let source = support();
    let outer = [
        [3.5, 0.125],
        [0.5, 0.125],
        [0.5, 0.875],
        [3.5, 0.875],
        [3.5, 0.125],
    ];
    let hole = [
        [0.125, 0.25],
        [0.375, 0.25],
        [0.375, 0.75],
        [0.125, 0.75],
        [0.125, 0.25],
    ];
    let report =
        tessellate_exact_periodic_trim_sheet(&source, &[&outer, &hole], 1e-8, 1_000_000, 4)
            .unwrap();
    let model = report.sheet.model.unwrap();
    let display = report.tessellation.unwrap();
    assert!(!display.built.report.closed);
    assert_eq!(display.built.report.non_manifold_edges, 0);
    assert_eq!(display.built.report.degenerate_triangles, 0);
    let expected_id = model.1.faces[0].to_string();
    assert!(
        display
            .topology_face_ids
            .unwrap()
            .iter()
            .all(|id| *id == expected_id)
    );

    let mesh = display.built.mesh;
    let mut edges = BTreeMap::new();
    for triangle in mesh.indices.chunks_exact(3) {
        for (a, b) in triangle
            .iter()
            .copied()
            .zip(triangle.iter().copied().cycle().skip(1))
        {
            *edges.entry([a.min(b), a.max(b)]).or_insert(0) += 1;
        }
    }
    let mut boundary: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
    for ([a, b], count) in edges {
        if count == 1 {
            boundary.entry(a).or_default().push(b);
            boundary.entry(b).or_default().push(a);
        }
    }
    let mut visited = BTreeSet::new();
    let mut loops = 0;
    for &start in boundary.keys() {
        if !visited.insert(start) {
            continue;
        }
        loops += 1;
        let mut pending = vec![start];
        while let Some(i) = pending.pop() {
            assert_eq!(boundary[&i].len(), 2);
            for &j in &boundary[&i] {
                if visited.insert(j) {
                    pending.push(j);
                }
            }
        }
    }
    assert_eq!(loops, 2);
}

#[test]
fn periodic_trim_display_keeps_a_refusal_and_rejects_invalid_tessellation_options() {
    let source = support();
    let bow = [[0., 0.], [1., 1.], [0., 1.], [1., 0.], [0., 0.]];
    let report =
        tessellate_exact_periodic_trim_sheet(&source, &[&bow], 1e-8, 1_000_000, 4).unwrap();
    assert!(report.sheet.refusal.is_some());
    assert!(report.tessellation.is_none());
    assert!(tessellate_exact_periodic_trim_sheet(&source, &[&bow], 1e-8, 1_000_000, 0).is_err());
}
