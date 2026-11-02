use brep_core::feature_ops::edge_partition::{self, Limits, seam};
#[path = "common/closed_periodic_source.rs"]
mod fixture;
#[test]
fn arbitrary_seam_moves_real_vertex_and_both_original_face_uses() {
    let root = fixture::source();
    let saved = root.clone();
    let r = seam::relocate(&root, 0, 0.37, "phase", Limits::default()).unwrap();
    let m = r.model();
    assert_eq!(m.edges.len(), 1);
    assert_eq!(m.vertices.len(), 1);
    assert_ne!(m.vertices[0].point, root.vertices[0].point);
    assert_eq!(m.1.faces, root.1.faces);
    assert_eq!(m.1.bodies, root.1.bodies);
    assert_ne!(m.1.edges, root.1.edges);
    assert_ne!(m.1.vertices, root.1.vertices);
    assert!(r.reverify().unwrap().trim_regions_valid);
    assert!((r.evidence().numerical_volume_mm3 - std::f64::consts::PI).abs() < 1e-6);
    let s = edge_partition::split(m, 0, &[1.25, 3.5], "after-phase", Limits::default()).unwrap();
    assert!(s.reverify().unwrap().boundary_complete);
    assert_eq!(root, saved);
}
#[test]
fn clamped_root_and_repeated_seam_keep_native_closure() {
    let mut root = fixture::source();
    let d = root.edges[0].curve.domain();
    root.edges[0].curve = root.edges[0].curve.trim(d[0], d[1]).unwrap();
    for l in &mut root.loops {
        let c = &mut l.coedges[0];
        c.pcurve = c.pcurve.trim(d[0], d[1]).unwrap();
    }
    let first = seam::relocate(&root, 0, 1.125, "first", Limits::default()).unwrap();
    let second = seam::relocate(first.model(), 0, 3.25, "second", Limits::default()).unwrap();
    assert!(second.reverify().unwrap().trim_regions_valid);
    assert_eq!(second.model().edges[0].vertices, [0, 0]);
}
#[test]
fn malformed_seams_and_exhausted_proofs_leave_original_unchanged() {
    let root = fixture::source();
    let saved = root.clone();
    for p in [f64::NAN, -1., 4., 10.] {
        assert!(seam::relocate(&root, 0, p, "bad", Limits::default()).is_err());
    }
    let tiny = Limits {
        max_cells: 1,
        ..Limits::default()
    };
    assert!(seam::relocate(&root, 0, 0.37, "tiny", tiny).is_err());
    assert_eq!(root, saved);
}
#[test]
fn reversed_world_and_independent_full_authored_clocks_survive_relocation() {
    let mut root = fixture::source();
    root.edges[0].curve = root.edges[0].curve.reverse().unwrap();
    for k in &mut root.edges[0].curve.knots {
        *k = -3. + 2.5 * *k;
    }
    for (i, l) in root.loops.iter_mut().enumerate() {
        let c = &mut l.coedges[0];
        c.reversed = true;
        for k in &mut c.pcurve.knots {
            *k = -11. + 10. * *k;
        }
        let mut g = [-3., 7.];
        let mut p = [-11., 29.];
        if i == 1 {
            g.reverse();
            p.reverse();
        }
        *c = c
            .clone()
            .with_trim(brep_topology::CoedgeTrim {
                curve_parameter: g,
                pcurve_parameter: p,
                periodic_lift: [[0; 2]; 2],
            })
            .unwrap();
    }
    let saved = root.clone();
    let result = seam::relocate(&root, 0, 0.125, "reversed-full", Limits::default()).unwrap();
    assert!(result.reverify().unwrap().trim_regions_valid);
    for (i, l) in result.model().loops.iter().enumerate() {
        let c = &l.coedges[0];
        assert!(c.reversed);
        let t = c.trim().unwrap();
        assert_eq!(t.curve_parameter[0] < t.curve_parameter[1], i == 0);
        assert_eq!(t.pcurve_parameter[0] < t.pcurve_parameter[1], i == 0);
    }
    assert_eq!(root, saved);
}
#[test]
fn partial_authored_relations_are_retained_as_real_clock_breaks() {
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
    let saved = root.clone();
    let relocated = seam::relocate(&root, 0, 0.37, "partial", Limits::default()).unwrap();
    assert_eq!(relocated.cycle().len(), 2);
    assert_eq!(relocated.relations().len(), 2);
    assert!(relocated.reverify().unwrap().trim_regions_valid);
    assert_eq!(root, saved);
}
#[test]
fn ordinary_unclamped_storage_also_relocates_the_actual_active_cycle() {
    let mut root = fixture::source();
    root.edges[0].curve.periodic = false;
    for l in &mut root.loops {
        l.coedges[0].pcurve.periodic = false;
    }
    let saved = root.clone();
    let result = seam::relocate(&root, 0, 0.375, "ordinary-cycle", Limits::default()).unwrap();
    assert!(result.reverify().unwrap().trim_regions_valid);
    assert!(!result.model().edges[0].curve.periodic);
    assert_eq!(root, saved);
}
