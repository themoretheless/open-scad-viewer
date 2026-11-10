use super::*;
fn fixture(points: &[[f64; 2]]) -> (Model, Vec<Segment>) {
    let m = crate::cuboid([0.; 3], [10., 8., 12.]).unwrap();
    let surface = &m.faces[0].surface;
    let path = (0..points.len())
        .map(|i| {
            let a = points[i];
            let b = points[(i + 1) % points.len()];
            Segment {
                curve: crate::line(
                    surface.evaluate(a[0], a[1]).unwrap().point.to_vec(),
                    surface.evaluate(b[0], b[1]).unwrap().point.to_vec(),
                ),
                pcurve: crate::line(a.to_vec(), b.to_vec()),
            }
        })
        .collect();
    (m, path)
}
fn square() -> (Model, Vec<Segment>) {
    fixture(&[[0.25, 0.25], [0.75, 0.25], [0.75, 0.75], [0.25, 0.75]])
}
#[test]
fn closed_face_split_preserves_closed_shell_and_reuses_each_new_edge_twice() {
    let (m, path) = square();
    let before = m.clone();
    let out = partition_closed(&m, 0, &path, 1e-8, Default::default()).unwrap();
    assert_eq!(out.model.faces.len(), m.faces.len() + 1);
    assert_eq!(out.model.faces[out.faces[0]].holes.len(), 1);
    assert!(out.model.faces[out.faces[1]].holes.is_empty());
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
    assert_eq!(out.model.faces[0].surface, m.faces[0].surface);
    assert_eq!(out.model.loops[m.faces[0].outer], m.loops[m.faces[0].outer]);
    out.model.validate().unwrap();
    assert_eq!(
        crate::watertight::inspect(&out.model, 100000)
            .unwrap()
            .status,
        crate::watertight::Closure::WithinTolerance
    );
    assert_eq!(m, before);
}
#[test]
fn clockwise_saved_path_is_normalized_without_snapping() {
    let (m, path) = square();
    let path = reverse(&path).unwrap();
    let out = partition_closed(&m, 0, &path, 1e-8, Default::default()).unwrap();
    assert!(out.path_reversed);
    assert_eq!(
        out.model.vertices[out.vertices[0]].point.as_slice(),
        path.last().unwrap().curve.control_points.last().unwrap()
    );
    out.model.validate().unwrap();
}
#[test]
fn nonexact_joins_bad_dimensions_and_self_crossing_paths_refuse_atomically() {
    let (m, mut path) = square();
    let before = m.clone();
    path[1].curve.control_points[0][0] += 1e-12;
    assert!(partition_closed(&m, 0, &path, 1e-8, Default::default()).is_err());
    let (_, mut path) = square();
    path[1].curve.control_points.clear();
    assert!(partition_closed(&m, 0, &path, 1e-8, Default::default()).is_err());
    let (_, path) = fixture(&[[0.25, 0.25], [0.75, 0.75], [0.25, 0.75], [0.75, 0.25]]);
    assert!(partition_closed(&m, 0, &path, 1e-8, Default::default()).is_err());
    assert_eq!(m, before);
}
#[test]
fn complete_source_windows_and_local_policies_survive_identity_rebuild() {
    let (mut m, path) = square();
    let e = m.1.edges[0];
    let v = m.1.vertices[0];
    m.1.local_tolerances_mm.insert(e, 1e-9);
    m.1.local_tolerances_mm.insert(v, 2e-9);
    m.1.sewed_edges.insert(e);
    let wire = m.faces[0].outer;
    for i in 0..m.loops[wire].coedges.len() {
        let c = m.loops[wire].coedges[i].clone();
        let t = brep_topology::CoedgeTrim {
            curve_parameter: m.edges[c.edge].curve.domain(),
            pcurve_parameter: c.pcurve.domain(),
            periodic_lift: [[0; 2]; 2],
        };
        m.loops[wire].coedges[i] = c.with_trim(t).unwrap();
    }
    let out = partition_closed(&m, 0, &path, 1e-8, Default::default()).unwrap();
    assert_eq!(out.model.1.local_tolerances_mm[&e], 1e-9);
    assert_eq!(out.model.1.local_tolerances_mm[&v], 2e-9);
    assert!(out.model.1.sewed_edges.contains(&e));
    assert_eq!(out.model.loops[wire], m.loops[wire]);
}
#[test]
fn closed_face_cut_checks_saved_lift_and_shared_budget() {
    let (m, path) = square();
    let before = m.clone();
    let out = partition_closed(&m, 0, &path, 1e-8, Default::default()).unwrap();
    let exact = ClosedCutLimits {
        cells: out.cells,
        pairs: out.pairs,
    };
    assert!(partition_closed(&m, 0, &path, 1e-8, exact).is_ok());
    assert!(partition_closed(&m, 0, &path, 1e-8, ClosedCutLimits { cells: 1, ..exact }).is_err());
    assert!(partition_closed(&m, 0, &path, 1e-8, ClosedCutLimits { pairs: 1, ..exact }).is_err());
    let wrong: Vec<_> = path
        .into_iter()
        .map(|mut s| {
            for p in &mut s.curve.control_points {
                p[2] += 0.01;
            }
            s
        })
        .collect();
    assert!(partition_closed(&m, 0, &wrong, 1e-8, Default::default()).is_err());
    assert_eq!(m, before);
}

#[test]
fn rounded_multispan_reversal_has_a_direct_original_reference_bound() {
    let (m, path) = square();
    let mut path = reverse(&path).unwrap();
    path[0].curve = path[0].curve.insert(0.1, 1).unwrap();
    path[0].pcurve = path[0].pcurve.insert(0.1, 1).unwrap();
    let out = partition_closed(&m, 0, &path, 1e-8, Default::default()).unwrap();
    assert!(out.path_reversed);
    assert!(!out.source_world_reference_exact);
    assert!(!out.source_uv_reference_exact);
    out.model.validate().unwrap();
}
#[test]
fn saved_uv_reference_refuses_interior_deformation_even_on_the_same_plane() {
    let source = crate::line(vec![0., 0.], vec![1., 0.]);
    let candidate =
        Curve::from_polyline(vec![vec![0., 0.], vec![0.5, 0.01], vec![1., 0.]]).unwrap();
    let mut work = Work::default();
    assert!(
        reference::reference(
            &candidate,
            &source,
            false,
            1e-8,
            &mut work,
            Default::default()
        )
        .is_err()
    );
    assert!(work.cells > 0);
}
