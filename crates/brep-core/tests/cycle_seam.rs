use brep_core::{
    Model,
    feature_ops::edge_partition::{self, Limits, seam},
};
#[cfg(feature = "codec")]
use value_codec::{Deserialize, Serialize};
#[path = "common/closed_periodic_source.rs"]
mod fixture;
fn source() -> Model {
    edge_partition::split(
        &fixture::source(),
        0,
        &[0.5, 1.5, 2.5],
        "initial-cycle",
        Limits::default(),
    )
    .unwrap()
    .model()
    .clone()
}
fn starts(m: &Model, v: usize) {
    for l in &m.loops {
        let c = &l.coedges[0];
        assert_eq!(m.edges[c.edge].vertices[usize::from(c.reversed)], v);
    }
}
fn traversal(r: &seam::Relocation) {
    assert_eq!(
        r.cycle(),
        r.traversal().iter().map(|u| u.edge).collect::<Vec<_>>()
    );
    let m = r.model();
    let mut v = r.seam_vertex();
    for u in r.traversal() {
        let e = &m.edges[u.edge];
        assert_eq!(e.vertices[usize::from(u.reversed)], v);
        v = e.vertices[usize::from(!u.reversed)];
    }
    assert_eq!(v, r.seam_vertex());
}
#[test]
fn a_partitioned_partial_clock_cycle_accepts_another_interior_seam() {
    let mut root = fixture::source();
    for l in &mut root.loops {
        let c = &mut l.coedges[0];
        *c = c
            .clone()
            .with_trim(brep_topology::CoedgeTrim {
                curve_parameter: [1., 3.],
                pcurve_parameter: [1., 3.],
                periodic_lift: [[0; 2]; 2],
            })
            .unwrap();
    }
    let first = seam::relocate(&root, 0, 2., "first", Limits::default()).unwrap();
    let saved = first.model().clone();
    let second = seam::relocate(
        first.model(),
        first.cycle()[0],
        2.75,
        "second",
        Limits::default(),
    )
    .unwrap();
    assert_eq!(second.model().edges.len(), 3);
    assert_eq!(second.model().vertices.len(), 3);
    assert_eq!(second.model().vertices[0], root.vertices[0]);
    assert_eq!(second.relations().len(), 2);
    for r in second.relations() {
        assert_eq!(r.fragments.len(), 2);
        for f in &r.fragments {
            assert!(
                second.model().loops[r.wire]
                    .coedges
                    .iter()
                    .any(|c| c.edge == f.edge && c.trim() == Some(f.transferred))
            );
        }
    }
    starts(second.model(), second.seam_vertex());
    traversal(&second);
    assert!(second.reverify().unwrap().trim_regions_valid);
    assert_eq!(first.model(), &saved);
    assert_eq!(second.model().1.faces, root.1.faces);
}
#[test]
fn endpoint_seams_reuse_actual_vertices_and_leave_all_edge_geometry_literal() {
    let root = source();
    let saved = root.clone();
    let selected = 1;
    let d = root.edges[selected].curve.domain();
    for (p, v) in [
        (d[0], root.edges[selected].vertices[0]),
        (d[1], root.edges[selected].vertices[1]),
    ] {
        let r = seam::relocate(&root, selected, p, "endpoint", Limits::default()).unwrap();
        assert_eq!(r.seam_vertex(), v);
        assert_eq!(r.model().edges, root.edges);
        assert_eq!(r.model().vertices, root.vertices);
        assert_eq!(r.model().1.edges, root.1.edges);
        assert_eq!(r.model().1.vertices, root.1.vertices);
        assert!(r.model().persistent_naming_complete());
        starts(r.model(), v);
        traversal(&r);
        assert!(r.reverify().unwrap().boundary_complete);
        assert!((r.evidence().numerical_volume_mm3 - std::f64::consts::PI).abs() < 1e-6);
    }
    assert_eq!(root, saved);
}
#[test]
fn mixed_world_edge_directions_have_an_explicit_closed_traversal() {
    let mut root = source();
    let e = 1;
    root.edges[e].curve = root.edges[e].curve.reverse().unwrap();
    root.edges[e].vertices.swap(0, 1);
    for l in &mut root.loops {
        for c in &mut l.coedges {
            if c.edge == e {
                c.reversed = !c.reversed;
            }
        }
    }
    root.validate().unwrap();
    let saved = root.clone();
    let r = seam::relocate(&root, e, 1., "mixed", Limits::default()).unwrap();
    assert_eq!(r.model().edges.len(), 5);
    assert!(r.traversal().iter().any(|u| u.reversed));
    assert!(!r.traversal()[0].reversed);
    starts(r.model(), r.seam_vertex());
    traversal(&r);
    assert!(r.reverify().unwrap().trim_regions_valid);
    assert_eq!(root, saved);
}
#[test]
fn transported_multi_edge_cycles_can_move_the_seam_repeatedly() {
    let root = source();
    let first = seam::relocate(&root, 2, 2., "one", Limits::default()).unwrap();
    #[cfg(feature = "codec")]
    let restored = Model::from_value(first.model().to_value()).unwrap();
    #[cfg(not(feature = "codec"))]
    let restored = first.model().clone();
    assert_eq!(&restored, first.model());
    let second = seam::relocate(&restored, 0, 0.25, "two", Limits::default()).unwrap();
    assert_eq!(second.model().edges.len(), 6);
    traversal(&second);
    starts(second.model(), second.seam_vertex());
    assert!(second.reverify().unwrap().boundary_complete);
    let d = second.model().edges[second.cycle()[0]].curve.domain();
    let third = seam::relocate(
        second.model(),
        second.cycle()[0],
        d[0],
        "reuse",
        Limits::default(),
    )
    .unwrap();
    assert_eq!(third.model(), second.model());
    assert!(third.reverify().unwrap().trim_regions_valid);
}
#[test]
fn unrelated_face_cycles_invalid_parameters_and_exhausted_work_cannot_publish() {
    let cube = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let cube_saved = cube.clone();
    assert!(seam::relocate(&cube, 0, 0.5, "not-common", Limits::default()).is_err());
    assert_eq!(cube, cube_saved);
    let root = source();
    let saved = root.clone();
    for p in [f64::NAN, -1., 20.] {
        assert!(seam::relocate(&root, 1, p, "bad", Limits::default()).is_err());
    }
    assert!(
        seam::relocate(
            &root,
            1,
            1.,
            "tiny",
            Limits {
                max_cells: 1,
                ..Limits::default()
            }
        )
        .is_err()
    );
    assert_eq!(root, saved);
}
