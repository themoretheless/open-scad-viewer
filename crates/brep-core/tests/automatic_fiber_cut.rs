use brep_core::{Coedge, Edge, Loop, Model, Vertex, face_cut};
use nurbs_core::{curve::Curve, ss_intersection as ss};
#[path = "../examples/support/fiber_cut_fixture.rs"]
mod fixture;
use fixture::pair;
#[test]
fn automatic_search_uses_original_domains_and_owned_roots() {
    let (source, cutter) = pair();
    let before = (source.clone(), cutter.clone());
    let out =
        face_cut::partition_fiber_at_face_automatic(&source, 0, &cutter, 0, 1e-8, 100000, 1000000)
            .unwrap();
    assert!(out.boundary_coverage.iter().all(|(_, r)| r.complete));
    assert!(out.cells <= 100000 && out.partition.exact_work <= 1000000);
    assert!(out.root_error_upper_mm <= source.tolerance_mm / 2.);
    out.partition.model.validate().unwrap();
    for &vertex in &out.endpoints {
        assert!(
            out.partition
                .edges
                .iter()
                .any(|&e| out.partition.model.edges[e].vertices.contains(&vertex))
        );
    }
    assert!(
        face_cut::partition_fiber_at_face_automatic(&source, 0, &cutter, 0, 1e-8, 32, 1000000,)
            .is_err()
    );
    assert_eq!((source, cutter), before);
}
#[test]
fn automatic_search_selects_v_after_transposing_the_original_chart() {
    let (mut source, cutter) = pair();
    let surface = &mut source.faces[0].surface;
    let rows = surface.control_points.len();
    let cols = surface.control_points[0].len();
    surface.control_points = (0..cols)
        .map(|v| {
            (0..rows)
                .map(|u| surface.control_points[u][v].clone())
                .collect()
        })
        .collect();
    surface.weights = (0..cols)
        .map(|v| (0..rows).map(|u| surface.weights[u][v]).collect())
        .collect();
    std::mem::swap(&mut surface.degree_u, &mut surface.degree_v);
    std::mem::swap(&mut surface.knots_u, &mut surface.knots_v);
    for c in &mut source.loops[0].coedges {
        for p in &mut c.pcurve.control_points {
            p.swap(0, 1);
        }
        c.pcurve = c.pcurve.reverse().unwrap();
        c.reversed = !c.reversed;
    }
    source.loops[0].coedges.reverse();
    source.shells[0].faces[0].reversed = !source.shells[0].faces[0].reversed;
    source.rebuild_topology_ids();
    source.validate().unwrap();
    let before = source.clone();
    let out =
        face_cut::partition_fiber_at_face_automatic(&source, 0, &cutter, 0, 1e-8, 100000, 1000000)
            .unwrap();
    assert!(out.boundary_coverage.iter().all(|(_, r)| r.complete));
    assert!(out.root_error_upper_mm <= source.tolerance_mm / 2.);
    out.partition.model.validate().unwrap();
    assert_eq!(source, before);
}
#[test]
fn general_curved_cut_binds_actual_original_edge_roots_before_atomic_partition() {
    let (source, cutter) = pair();
    let before = (source.clone(), cutter.clone());
    let box4 = [[0., 1.], [0.375, 0.625], [0.2, 0.55], [0.3, 0.7]];
    let out = face_cut::partition_fiber_at_face(
        &source,
        0,
        &cutter,
        0,
        box4,
        ss::FiberAxis::FirstU,
        1e-8,
        100000,
        1000000,
    )
    .unwrap();
    println!(
        "fiber cut cells={} evaluations={} edges={} root_error={}",
        out.cells,
        out.evaluations,
        out.partition.edges.len(),
        out.root_error_upper_mm
    );
    assert!(out.cells <= 100000 && out.partition.exact_work <= 1000000);
    assert!(out.root_error_upper_mm <= source.tolerance_mm / 2.);
    assert!(out.trace_pieces > 1);
    assert!(out.partition.edges.len() < out.trace_pieces);
    assert!(out.boundary_coverage.iter().all(|(_, r)| r.complete));
    for &edge in &out.partition.edges {
        assert_eq!(
            out.partition
                .model
                .loops
                .iter()
                .flat_map(|l| &l.coedges)
                .filter(|c| c.edge == edge)
                .count(),
            2
        );
    }
    assert_eq!(
        out.partition.model.edges[*out.partition.edges.first().unwrap()].vertices[0],
        out.endpoints[0]
    );
    assert_eq!(
        out.partition.model.edges[*out.partition.edges.last().unwrap()].vertices[1],
        out.endpoints[1]
    );
    out.partition.model.validate().unwrap();
    let restored: Model =
        value_codec::from_str(&value_codec::to_string(&out.partition.model).unwrap()).unwrap();
    assert_eq!(restored, out.partition.model);
    assert!(
        face_cut::partition_fiber_at_face(
            &source,
            0,
            &cutter,
            0,
            box4,
            ss::FiberAxis::FirstU,
            1e-8,
            32,
            1000000
        )
        .is_err()
    );
    let mut wrong = box4;
    wrong[1] = [0.1, 0.2];
    assert!(
        face_cut::partition_fiber_at_face(
            &source,
            0,
            &cutter,
            0,
            wrong,
            ss::FiberAxis::FirstU,
            1e-8,
            100000,
            1000000
        )
        .is_err()
    );
    assert_eq!((source, cutter), before);
}

#[test]
fn a_hole_in_the_actual_curved_cutter_cannot_be_treated_as_its_carrier() {
    let (source, mut cutter) = pair();
    let uv = [[0.34, 0.45], [0.34, 0.55], [0.39, 0.55], [0.39, 0.45]];
    let vertex_base = cutter.vertices.len();
    let edge_base = cutter.edges.len();
    let mut coedges = Vec::new();
    for i in 0..4 {
        let pcurve = Curve::from_polyline(vec![uv[i].to_vec(), uv[(i + 1) % 4].to_vec()]).unwrap();
        let lifted = nurbs_core::curve_surface_agreement::lift(
            &pcurve,
            &cutter.faces[0].surface,
            None,
            1e-9,
            100000,
        )
        .unwrap();
        let curve = lifted.curve.unwrap();
        cutter.vertices.push(Vertex {
            point: curve.control_points[0].clone().try_into().unwrap(),
        });
        cutter.edges.push(Edge {
            curve,
            vertices: [vertex_base + i, vertex_base + (i + 1) % 4],
            degenerate: false,
        });
        coedges.push(Coedge::new(
            edge_base + i,
            false,
            pcurve,
        ));
    }
    let hole = cutter.loops.len();
    cutter.loops.push(Loop { coedges });
    cutter.faces[0].holes.push(hole);
    cutter.rebuild_topology_ids();
    cutter.validate().unwrap();
    let before = (source.clone(), cutter.clone());
    let result = face_cut::partition_fiber_at_face(
        &source,
        0,
        &cutter,
        0,
        [[0., 1.], [0.375, 0.625], [0.2, 0.55], [0.3, 0.7]],
        ss::FiberAxis::FirstU,
        1e-8,
        100000,
        1000000,
    );
    assert!(result.is_err());
    assert!(
        face_cut::partition_fiber_at_face_automatic(&source, 0, &cutter, 0, 1e-8, 100000, 1000000,)
            .is_err()
    );
    assert_eq!((source, cutter), before);
}
