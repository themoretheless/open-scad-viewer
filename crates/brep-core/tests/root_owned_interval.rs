#[path = "../examples/support/nurbs_contact_stock.rs"]
mod rational;
use brep_core::{
    Model, edge_partition::root_interval as roots, feature_ops::edge_partition::Limits,
};
fn cutter(z: f64) -> (Model, usize) {
    let m = brep_core::cuboid([-20., -20., z], [20., 20., z + 0.5]).unwrap();
    let f = m
        .faces
        .iter()
        .position(|f| f.surface.control_points.iter().flatten().all(|p| p[2] == z))
        .unwrap();
    (m, f)
}
fn check(source: &Model, edge: usize, za: f64, zb: f64, key: &str) -> roots::RootInterval {
    let (a, fa) = cutter(za);
    let (b, fb) = cutter(zb);
    let before = source.clone();
    let result = roots::between_faces(
        source,
        edge,
        [(&a, fa, 0), (&b, fb, 0)],
        key,
        1e-8,
        Limits::default(),
    )
    .unwrap();
    assert_eq!(source, &before);
    assert!(result.cells() <= Limits::default().max_cells);
    assert!(
        result
            .root_error_upper()
            .iter()
            .all(|e| *e <= source.tolerance_mm)
    );
    let model = result.model();
    let middle = &model.edges[result.middle_edge()];
    assert_eq!(middle.vertices, result.endpoints());
    assert!((model.vertices[result.endpoints()[0]].point[2] - za).abs() <= source.tolerance_mm);
    assert!((model.vertices[result.endpoints()[1]].point[2] - zb).abs() <= source.tolerance_mm);
    let mut uses = 0;
    for f in &model.faces {
        for w in std::iter::once(f.outer).chain(f.holes.iter().copied()) {
            for c in &model.loops[w].coedges {
                if c.edge != result.middle_edge() {
                    continue;
                }
                uses += 1;
                let proof = nurbs_core::curve_surface_agreement::verify(
                    &middle.curve,
                    &c.pcurve,
                    &f.surface,
                    c.reversed,
                    source.tolerance_mm,
                    10000,
                )
                .unwrap();
                assert_eq!(
                    proof.status,
                    nurbs_core::curve_surface_agreement::Status::WithinTolerance
                );
            }
        }
    }
    assert_eq!(uses, 2);
    assert!(result.partition().evidence().boundary_complete);
    result.reverify().unwrap();
    eprintln!(
        "root_case={key} middle_domain={:?} root_errors_mm={:?} shared_vertices={:?} cells={}",
        result.model().edges[result.middle_edge()].curve.domain(),
        result.root_error_upper(),
        result.endpoints(),
        result.cells()
    );
    result
}
#[test]
fn both_original_roots_own_the_middle_edge_and_both_neighboring_faces() {
    let source = brep_core::cuboid([0.; 3], [4., 5., 10.]).unwrap();
    let edge = source
        .edges
        .iter()
        .position(|e| {
            source.vertices[e.vertices[0]].point[2] == 0.
                && source.vertices[e.vertices[1]].point[2] == 10.
        })
        .unwrap();
    let first = check(&source, edge, 2., 8., "root-middle");
    let second = check(
        first.model(),
        first.middle_edge(),
        4.,
        6.,
        "nested-root-middle",
    );
    assert_eq!(second.model().faces, source.faces);
}
#[test]
fn rational_shared_edge_with_independent_nonunit_clocks_keeps_root_end_ownership() {
    let mut source = rational::build().source;
    let edge = source
        .edges
        .iter()
        .position(|e| {
            e.curve.degree == 2
                && (source.vertices[e.vertices[0]].point[2]
                    - source.vertices[e.vertices[1]].point[2])
                    .abs()
                    > 9.
        })
        .unwrap();
    source.edges[edge].curve = source.edges[edge].curve.refine(&[0.25, 0.5, 0.75]).unwrap();
    for k in &mut source.edges[edge].curve.knots {
        *k = -3. + 10. * *k;
    }
    for (w, wire) in source.loops.iter_mut().enumerate() {
        for c in &mut wire.coedges {
            if c.edge == edge {
                for k in &mut c.pcurve.knots {
                    *k = -11. + w as f64 * 2. + 40. * *k;
                }
            }
        }
    }
    source.rebuild_topology_ids();
    source.validate().unwrap();
    let start = source.vertices[source.edges[edge].vertices[0]].point[2];
    let (za, zb) = if start == 0. { (2., 8.) } else { (8., 2.) };
    let first = check(&source, edge, za, zb, "rational-root-middle");
    let (za, zb) = if start == 0. { (4., 6.) } else { (6., 4.) };
    check(
        first.model(),
        first.middle_edge(),
        za,
        zb,
        "rational-nested-roots",
    );
}
#[test]
fn reversed_missing_and_exhausted_roots_refuse_without_partial_source_edits() {
    let source = brep_core::cuboid([0.; 3], [4., 5., 10.]).unwrap();
    let before = source.clone();
    let edge = source
        .edges
        .iter()
        .position(|e| {
            source.vertices[e.vertices[0]].point[2] == 0.
                && source.vertices[e.vertices[1]].point[2] == 10.
        })
        .unwrap();
    let (a, fa) = cutter(2.);
    let (b, fb) = cutter(8.);
    for cutters in [
        [(&b, fb, 0), (&a, fa, 0)],
        [(&a, fa, 0), (&a, fa, 0)],
        [(&a, fa, 1), (&b, fb, 0)],
    ] {
        assert!(
            roots::between_faces(&source, edge, cutters, "bad-roots", 1e-8, Limits::default())
                .is_err()
        );
    }
    assert!(
        roots::between_faces(
            &source,
            edge,
            [(&a, fa, 0), (&b, fb, 0)],
            "exhausted",
            1e-8,
            Limits {
                max_cells: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert_eq!(source, before);
}
#[test]
fn existing_single_root_entry_uses_the_same_shared_clock_path_on_a_cropped_rational_edge() {
    let mut source = rational::build().source;
    let edge = source
        .edges
        .iter()
        .position(|e| {
            e.curve.degree == 2
                && (source.vertices[e.vertices[0]].point[2]
                    - source.vertices[e.vertices[1]].point[2])
                    .abs()
                    > 9.
        })
        .unwrap();
    // The subsequent z=5 cut is exactly at an ORIGINAL interior knot.
    source.edges[edge].curve = source.edges[edge].curve.refine(&[0.25, 0.5, 0.75]).unwrap();
    source.rebuild_topology_ids();
    source.validate().unwrap();
    let start = source.vertices[source.edges[edge].vertices[0]].point[2];
    let (za, zb) = if start == 0. { (2., 8.) } else { (8., 2.) };
    let first = check(&source, edge, za, zb, "single-root-parent");
    let (cut, face) = cutter(5.);
    let single = brep_core::edge_partition::partition_at_face(
        first.model(),
        first.middle_edge(),
        &cut,
        face,
        0,
        1e-8,
        100000,
    )
    .unwrap();
    assert!(single.result.root_position_error_upper <= source.tolerance_mm);
    let vertex = single.result.partition.vertex;
    for edge in single.result.partition.child_edges {
        assert!(
            single.result.partition.model.edges[edge]
                .vertices
                .contains(&vertex)
        );
        assert_eq!(
            single
                .result
                .partition
                .model
                .loops
                .iter()
                .flat_map(|w| &w.coedges)
                .filter(|c| c.edge == edge)
                .count(),
            2
        );
    }
}
