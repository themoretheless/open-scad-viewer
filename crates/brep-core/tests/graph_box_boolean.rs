use brep_core::{
    Model,
    minkowski3d::curved::{certify_graph_box_boolean, verify_graph_box_boolean},
};
fn source() -> Model {
    let mut a = brep_core::cuboid([0.; 3], [1.; 3]).unwrap();
    let old = a.faces[1].surface.clone();
    let ranges = [
        [
            old.knots_u[old.degree_u],
            old.knots_u[old.control_points.len()],
        ],
        [
            old.knots_v[old.degree_v],
            old.knots_v[old.control_points[0].len()],
        ],
    ];
    let mut top = old.clone();
    top.degree_u = 2;
    top.degree_v = 2;
    top.knots_u = vec![0., 0., 0., 1., 1., 1.];
    top.knots_v = top.knots_u.clone();
    top.weights = vec![vec![1.; 3]; 3];
    top.control_points = (0..3)
        .map(|i| {
            (0..3)
                .map(|j| {
                    let mut p = (0..3)
                        .map(|k| {
                            old.control_points[0][0][k]
                                + (old.control_points[1][0][k] - old.control_points[0][0][k])
                                    * i as f64
                                    / 2.
                                + (old.control_points[0][1][k] - old.control_points[0][0][k])
                                    * j as f64
                                    / 2.
                        })
                        .collect::<Vec<_>>();
                    if i == 1 && j == 1 {
                        p[2] += 0.25;
                    }
                    p
                })
                .collect()
        })
        .collect();
    let wire = a.faces[1].outer;
    for c in &mut a.loops[wire].coedges {
        for p in &mut c.pcurve.control_points {
            for k in 0..2 {
                p[k] = (p[k] - ranges[k][0]) / (ranges[k][1] - ranges[k][0]);
            }
        }
    }
    a.faces[1].surface = top;
    a.rebuild_topology_ids();
    a.validate().unwrap();
    a
}
fn tool() -> Model {
    brep_core::cuboid([0.1875, 0.1875, 0.75], [0.3125, 0.3125, 2.]).unwrap()
}
#[test]
fn crossing_curved_face_splits_regions_and_authors_shared_closed_seams_in_standard_boolean() {
    let a = source();
    let b = tool();
    let original = a.clone();
    for op in ["difference", "intersection"] {
        let r = certify_graph_box_boolean(&a, &b, op, 1, 0).unwrap();
        verify_graph_box_boolean(&a, &b, r.model(), r.certificate()).unwrap();
        r.model().validate_continuous(32768).unwrap();
        let standard = brep_core::operations::boolean(&a, &b, op).unwrap();
        assert_eq!(&standard, r.model());
        assert_eq!(
            r.certificate().geometry.maximum_boundary_error_mm,
            [String::from("0"), String::from("1")]
        );
        assert_eq!(r.model().bodies.len(), 1);
        assert!(r.certificate().sewn_edges > 0);
        if op == "difference" {
            assert!(
                r.model()
                    .faces
                    .iter()
                    .any(|f| f.surface == a.faces[1].surface && !f.holes.is_empty())
            );
            assert!(r.model().faces.len() > a.faces.len());
        } else {
            assert_eq!(
                brep_core::operations::boolean(&b, &a, op).unwrap(),
                standard
            );
        }
    }
    assert_eq!(a, original);
}
#[test]
fn source_operation_result_and_material_column_are_bound_by_replay() {
    let a = source();
    let b = tool();
    let r = certify_graph_box_boolean(&a, &b, "difference", 1, 0).unwrap();
    let mut cert = r.certificate().clone();
    cert.column[2][0] = [String::from("0"), String::from("1")];
    assert!(verify_graph_box_boolean(&a, &b, r.model(), &cert).is_err());
    let mut cert = r.certificate().clone();
    cert.sewn_edges += 1;
    assert!(verify_graph_box_boolean(&a, &b, r.model(), &cert).is_err());
    let mut cert = r.certificate().clone();
    cert.operation = "intersection".into();
    assert!(verify_graph_box_boolean(&a, &b, r.model(), &cert).is_err());
    let mut cert = r.certificate().clone();
    cert.geometry.pocket.selected.clear();
    assert!(verify_graph_box_boolean(&a, &b, r.model(), &cert).is_err());
    let mut changed = r.model().clone();
    changed.vertices[0].point[2] += 0.01;
    assert!(verify_graph_box_boolean(&a, &b, &changed, r.certificate()).is_err());
    let other = brep_core::cuboid([0.1875, 0.1875, 0.5], [0.3125, 0.3125, 2.]).unwrap();
    assert!(verify_graph_box_boolean(&a, &other, r.model(), r.certificate()).is_err());
}
#[test]
fn unproved_contact_and_missing_tool_coverage_refuse() {
    let a = source();
    let b = tool();
    for (lo, hi) in [
        ([0.1875, 0.1875, 0.75], [0.3125, 0.3125, 1.01]),
        ([0.1875, 0.1875, -0.5], [0.3125, 0.3125, 2.]),
        ([-0.1, 0.1875, 0.75], [0.3125, 0.3125, 2.]),
        ([0.1875, 0.1875, 1.1], [0.3125, 0.3125, 2.]),
    ] {
        let t = brep_core::cuboid(lo, hi).unwrap();
        assert!(certify_graph_box_boolean(&a, &t, "difference", 1, 0).is_err());
    }
    assert!(certify_graph_box_boolean(&a, &b, "difference", 0, 0).is_err());
    for op in ["union", "xor"] {
        assert!(certify_graph_box_boolean(&a, &b, op, 1, 0).is_err());
    }
}

#[test]
fn another_material_layer_in_the_tool_column_cannot_be_missed() {
    let base = brep_core::cuboid([0.; 3], [1., 1., 0.5]).unwrap();
    let bridge = brep_core::cuboid([0.875, 0., 0.25], [1., 1., 1.75]).unwrap();
    let roof = brep_core::cuboid([0., 0., 1.5], [1., 1., 2.]).unwrap();
    let connected = brep_core::operations::boolean(&base, &bridge, "union").unwrap();
    let body = brep_core::operations::boolean(&connected, &roof, "union").unwrap();
    assert_eq!(body.bodies.len(), 1);
    let cutter = brep_core::cuboid([0.1875, 0.1875, 0.25], [0.3125, 0.3125, 3.]).unwrap();
    let faces = body
        .faces
        .iter()
        .enumerate()
        .filter(|(_, f)| {
            f.surface
                .control_points
                .iter()
                .flatten()
                .all(|p| p[2] == 0.5)
        })
        .map(|(i, _)| i)
        .collect::<Vec<_>>();
    assert!(!faces.is_empty());
    for face in faces {
        assert!(certify_graph_box_boolean(&body, &cutter, "difference", face, 0).is_err());
    }
}

#[test]
fn another_box_cut_uses_the_saved_trimmed_model_and_preserves_the_first_hole() {
    let a = source();
    let first = brep_core::operations::boolean(&a, &tool(), "difference").unwrap();
    let second_tool = brep_core::cuboid([0.375, 0.1875, 0.8125], [0.4375, 0.25, 2.]).unwrap();
    let next = brep_core::operations::boolean(&first, &second_tool, "difference").unwrap();
    next.validate_continuous(65536).unwrap();
    let roof = next
        .faces
        .iter()
        .find(|f| f.surface == a.faces[1].surface)
        .unwrap();
    assert_eq!(roof.holes.len(), 2);
    assert_eq!(
        first
            .faces
            .iter()
            .find(|f| f.surface == a.faces[1].surface)
            .unwrap()
            .holes
            .len(),
        1
    );
}
