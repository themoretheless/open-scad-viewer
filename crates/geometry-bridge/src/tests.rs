use super::*;
fn plane() -> Surface {
    Surface {
        degree_u: 1,
        degree_v: 1,
        knots_u: vec![0., 0., 1., 1.],
        knots_v: vec![0., 0., 1., 1.],
        control_points: vec![
            vec![vec![0., 0., 0.], vec![0., 10., 0.]],
            vec![vec![10., 0., 0.], vec![10., 10., 0.]],
        ],
        weights: vec![vec![1., 1.], vec![1., 1.]],
        periodic_u: false,
        periodic_v: false,
    }
}

#[test]
fn brep_transport_carries_authoritative_canonical_change_set() {
    let value = dispatch(json!({
        "op": "brep_nurbs_box",
        "min": [0., 0., 0.],
        "max": [1., 2., 3.],
    }))
    .unwrap();
    let ids = &value["topologyIds"];
    assert_eq!(ids["changeSet"]["schema"].as_u64(), Some(1));
    assert_eq!(
        ids["changeSet"]["nodes"].as_array().unwrap().len(),
        8 + 12 + 6 + 6 + 1 + 1
    );
    assert!(
        ids["faces"]
            .as_array()
            .unwrap()
            .iter()
            .all(|id| id.as_str().is_some_and(|id| {
                id.starts_with("f:")
                    && id.len() == 34
                    && id[2..]
                        .bytes()
                        .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
            }))
    );
}
#[test]
fn nurbs_to_polygons_and_back_to_exact_boundary_curves() {
    let source = plane();
    let sampled = tessellate_nurbs(
        &source,
        &Options {
            segments_u: 4,
            segments_v: 3,
            trim: None,
            max_triangles: None,
        },
    )
    .unwrap();
    let loops = sampled.mesh.boundary_loops().unwrap();
    let curves = boundary_curves(&sampled.mesh).unwrap();
    assert_eq!(curves.len(), 1);
    assert_eq!(curves[0].degree, 1);
    assert!(!curves[0].periodic);
    for (i, index) in loops[0].iter().enumerate() {
        assert_eq!(
            curves[0].evaluate(i as f64).unwrap().point,
            sampled.mesh.point(*index).unwrap().to_vec()
        );
    }
    let solid = sampled.mesh.thicken([0., 0., 2.]).unwrap();
    assert!(solid.report.closed);
    assert!((solid.report.signed_volume_mm3 - 200.).abs() < 1e-9);
    assert!(solid.mesh.uv.is_none());
    assert_eq!(source.control_points[0][0], vec![0., 0., 0.]);
}
#[test]
fn mesh_boundary_can_construct_a_new_nurbs_surface() {
    let source = plane();
    let mesh = tessellate_nurbs(
        &source,
        &Options {
            segments_u: 2,
            segments_v: 2,
            trim: None,
            max_triangles: None,
        },
    )
    .unwrap();
    let curves = boundary_curves(&mesh.mesh).unwrap();
    let walls = nurbs_core::surface::extrude(&curves[0], [0., 0., 3.]).unwrap();
    let result = tessellate_nurbs(
        &walls,
        &Options {
            segments_u: 8,
            segments_v: 2,
            trim: None,
            max_triangles: None,
        },
    )
    .unwrap();
    assert_eq!(result.report.non_manifold_edges, 0);
    assert!(result.report.parameter_seams_welded.unwrap().u);
}
#[test]
fn invalid_surface_is_rejected_before_polygons_are_created() {
    let mut source = plane();
    source.weights[0][0] = 0.;
    assert!(
        tessellate_nurbs(
            &source,
            &Options {
                segments_u: 2,
                segments_v: 2,
                trim: None,
                max_triangles: None
            }
        )
        .is_err()
    );
}

#[test]
fn brep_tessellation_preserves_faces_and_interchanges_both_kernels() {
    let model = brep_core::cuboid([0.; 3], [2., 3., 4.]).unwrap();
    for segments in [1, 2, 4] {
        let mesh = crate::brep::nurbs(&model, segments).unwrap();
        assert!(mesh.built.report.closed);
        assert!((mesh.built.report.signed_volume_mm3 - 24.).abs() < 1e-8);
        assert_eq!(
            mesh.face_ids
                .iter()
                .copied()
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            6
        );
        let polygon =
            polygon_core::solid::brep::from_mesh(&mesh.built.mesh, Some(&mesh.face_ids)).unwrap();
        assert_eq!(polygon.faces.len(), 6);
        assert_eq!(polygon.bodies.len(), 1);
        let back = crate::brep::polygons(&polygon).unwrap();
        assert!(back.built.report.closed);
        assert!((back.built.report.signed_volume_mm3 - 24.).abs() < 1e-8);
    }
}

#[test]
fn brep_operations_dispatch_to_closed_tessellated_solids() {
    let a = brep_core::cuboid([0.; 3], [2., 2., 2.]).unwrap();
    let b = brep_core::cuboid([1., 1., 0.], [3., 3., 2.]).unwrap();
    for operation in ["union", "difference", "intersection"] {
        let result = brep_core::boolean(&a, &b, operation).unwrap();
        let tessellation = crate::brep::nurbs(&result, 2).unwrap();
        assert!(tessellation.built.report.closed);
        assert_eq!(tessellation.built.report.non_manifold_edges, 0);
    }
    for result in [
        brep_core::chamfer(&a, 0, 0.25).unwrap(),
        brep_core::fillet(&a, 0, 0.25, 8).unwrap(),
    ] {
        let tessellation = crate::brep::nurbs(&result, 2).unwrap();
        assert!(tessellation.built.report.closed);
        assert_eq!(tessellation.built.report.orientation_conflicts, 0);
    }

    let separated = brep_core::cuboid([4., 0., 0.], [5., 1., 1.]).unwrap();
    let multi_body = brep_core::boolean(&a, &separated, "union").unwrap();
    assert_eq!(multi_body.bodies.len(), 2);
    let tessellation = crate::brep::nurbs(&multi_body, 2).unwrap();
    assert!(tessellation.built.report.closed);
    assert!((tessellation.built.report.signed_volume_mm3 - 9.).abs() < 1e-8);
}

#[test]
fn mesh_section_and_toolpath_ops_cut_a_box() {
    let mesh = polygon_core::solid::primitives::cube([10., 10., 10.], false).unwrap();
    let mesh_value = value_codec::to_value(&mesh).unwrap();
    let section = dispatch(json!({
        "op": "mesh_section",
        "mesh": mesh_value,
        "z": 1.0,
    }))
    .unwrap();
    assert!(section["contours"].as_array().unwrap().len() >= 1);

    let toolpaths = dispatch(json!({
        "op": "mesh_toolpaths",
        "mesh": mesh_value,
        "zMin": 0.0,
        "zMax": 2.0,
        "layerHeightMm": 0.5,
        "wallCount": 1,
    }))
    .unwrap();
    assert!(toolpaths["layers"].as_array().unwrap().len() >= 1);

    let gcode = dispatch(json!({
        "op": "mesh_gcode",
        "mesh": mesh_value,
        "zMin": 0.0,
        "zMax": 1.0,
        "layerHeightMm": 0.5,
        "wallCount": 1,
    }))
    .unwrap();
    assert!(gcode["gcode"].as_str().unwrap().contains("G1"));
}

#[test]
fn analytic_step_export_import_roundtrip() {
    let model = encode(brep_core::cylinder(2., 4.).unwrap()).unwrap();
    let exported = dispatch(json!({
        "op": "brep_nurbs_export_step",
        "model": model,
    }))
    .unwrap();
    let text = exported["text"].as_str().unwrap();
    assert!(text.contains("CYLINDRICAL_SURFACE"));
    assert!(text.contains("CIRCLE"));
    assert!(text.contains("ADVANCED_FACE"));
    assert!(!text.contains("OSCAD_SOLID"));
    assert!(!text.contains("FACETED_BREP"));
    assert_eq!(
        exported["certificate"]["capability"].as_str(),
        Some("step-interchange/1")
    );
    assert_eq!(exported["certificate"]["complete"], true);
    let imported = dispatch(json!({
        "op": "brep_nurbs_import_step",
        "text": text,
    }))
    .unwrap();
    assert_eq!(imported["certificate"]["complete"], true);
    assert!(imported["model"]["faces"].as_array().unwrap().len() >= 6);
    let faceted = "ISO-10303-21;\nDATA;\n#1=FACETED_BREP('',#2);\nENDSEC;\nEND-ISO-10303-21;\n";
    assert!(
        dispatch(json!({"op": "brep_nurbs_import_step", "text": faceted})).is_err(),
        "faceted must refuse on analytic import"
    );
}

#[test]
fn step_successor_bridge_reports_identity_preservation_and_loss() {
    let model = encode(brep_core::cuboid([0., 0., 0.], [2., 3., 4.]).unwrap()).unwrap();
    let exported = dispatch(json!({
        "op": "brep_nurbs_export_step_v2",
        "model": model,
    }))
    .unwrap();
    assert_eq!(
        exported["certificate"]["capability"].as_str(),
        Some("step-interchange/2")
    );
    assert_eq!(exported["identity"]["preserved"], true);
    let text = exported["text"].as_str().unwrap();
    assert!(text.contains("GLOBAL_UNIT_ASSIGNED_CONTEXT"));
    assert!(text.contains("OSCAD_TOPO/2|face|"));
    let imported = dispatch(json!({
        "op": "brep_nurbs_import_step_v2",
        "text": text,
    }))
    .unwrap();
    assert_eq!(imported["identity"]["preserved"], true);

    let stripped = text
        .lines()
        .map(|line| {
            if let Some(start) = line.find("'OSCAD_TOPO/2|") {
                let end = line[start + 1..].find('\'').unwrap() + start + 1;
                format!("{}''{}", &line[..start], &line[end + 1..])
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    let external = dispatch(json!({
        "op": "brep_nurbs_import_step_v2",
        "text": stripped,
    }))
    .unwrap();
    assert_eq!(external["identity"]["preserved"], false);
    assert!(external["identity"]["createdCount"].as_u64().unwrap() > 0);
}

#[test]
fn audited_feature_successors_cross_the_bridge_with_certificates() {
    let model = brep_core::cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
    let edges = model
        .edges
        .iter()
        .enumerate()
        .filter_map(|(index, edge)| {
            let a = model.vertices[edge.vertices[0]].point;
            let b = model.vertices[edge.vertices[1]].point;
            ((a[0] - b[0]).abs() <= 1e-12 && (a[1] - b[1]).abs() <= 1e-12)
                .then_some(index)
        })
        .take(2)
        .collect::<Vec<_>>();
    let fillet = dispatch(json!({
        "op":"brep_nurbs_audited_multi_edge_fillet",
        "model":encode(model).unwrap(),
        "edges":edges,
        "radius":0.5
    }))
    .unwrap();
    assert_eq!(
        fillet["certificate"]["capability"].as_str(),
        Some("analytic-multi-edge-fillet/1")
    );
    assert_eq!(fillet["certificate"]["complete"], true);
    assert_eq!(fillet["audit"]["ok"], true);
    assert_eq!(fillet["namingComplete"], true);

    let sweep = dispatch(json!({
        "op":"brep_nurbs_audited_parallel_frame_sweep",
        "profile":[[0.,0.],[2.,0.],[2.,1.],[0.,1.]],
        "path":[[3.,-1.,0.],[3.,-1.,4.]],
        "frameLaw":"rmf"
    }))
    .unwrap();
    assert_eq!(
        sweep["certificate"]["capability"].as_str(),
        Some("exact-parallel-frame-sweep/1")
    );
    assert_eq!(sweep["audit"]["ok"], true);
    assert_eq!(sweep["namingComplete"], true);
    assert!(
        dispatch(json!({
            "op":"brep_nurbs_audited_parallel_frame_sweep",
            "profile":[[0.,0.],[2.,0.],[2.,1.],[0.,1.]],
            "path":[[0.,0.,0.],[0.,0.,2.],[0.,1.,4.]],
            "frameLaw":"rmf"
        }))
        .is_err()
    );
}

#[test]
fn freeform_nurbs_step_export_import_roundtrip() {
    let mut cps = Vec::new();
    for y in 0..4 {
        cps.push(
            (0..4)
                .map(|x| vec![x as f64, y as f64, 0.05 * (x + y) as f64])
                .collect::<Vec<_>>(),
        );
    }
    let surface = nurbs_core::surface::Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: cps,
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    let model = encode(brep_core::bicubic_open_face(surface).unwrap()).unwrap();
    let exported = dispatch(json!({
        "op": "brep_nurbs_export_step_freeform",
        "model": model,
    }))
    .unwrap();
    let text = exported["text"].as_str().unwrap();
    assert!(text.contains("B_SPLINE_SURFACE_WITH_KNOTS"));
    assert!(text.contains("OPEN_SHELL"));
    assert!(!text.contains("MANIFOLD_SOLID_BREP"));
    assert!(!text.contains("OSCAD_SOLID"));
    assert_eq!(
        exported["certificate"]["capability"].as_str(),
        Some("nurbs-step-bicubic-face/1")
    );
    let imported = dispatch(json!({
        "op": "brep_nurbs_import_step_freeform",
        "text": text,
    }))
    .unwrap();
    assert_eq!(imported["certificate"]["complete"], true);
    assert_eq!(imported["model"]["faces"].as_array().unwrap().len(), 1);
    assert!(
        dispatch(json!({
            "op": "brep_nurbs_export_step_freeform",
            "model": encode(brep_core::cuboid([0.,0.,0.],[1.,1.,1.]).unwrap()).unwrap(),
        }))
        .is_err(),
        "constructor solid must refuse on freeform export"
    );
}

#[test]
fn freeform_nurbs_step_trimmed_and_solid_bridge() {
    let mut cps = Vec::new();
    for y in 0..4 {
        cps.push(
            (0..4)
                .map(|x| vec![x as f64, y as f64, 0.02 * (x + y) as f64])
                .collect::<Vec<_>>(),
        );
    }
    let surface = nurbs_core::surface::Surface {
        degree_u: 3,
        degree_v: 3,
        knots_u: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        knots_v: vec![0., 0., 0., 0., 1., 1., 1., 1.],
        control_points: cps,
        weights: vec![vec![1.; 4]; 4],
        periodic_u: false,
        periodic_v: false,
    };
    let hole = [[0.2, 0.2], [0.8, 0.2], [0.8, 0.8], [0.2, 0.8]];
    let trimmed = encode(brep_core::bicubic_trimmed_face(surface, hole).unwrap()).unwrap();
    let exported = dispatch(json!({
        "op": "brep_nurbs_export_step_trimmed",
        "model": trimmed,
    }))
    .unwrap();
    assert!(exported["text"].as_str().unwrap().contains("FACE_BOUND"));
    assert_eq!(
        exported["certificate"]["capability"].as_str(),
        Some("nurbs-step-trimmed-bicubic/1")
    );
    let imported = dispatch(json!({
        "op": "brep_nurbs_import_step_trimmed",
        "text": exported["text"],
    }))
    .unwrap();
    assert_eq!(
        imported["model"]["faces"][0]["holes"]
            .as_array()
            .unwrap()
            .len(),
        1
    );

    let solid =
        encode(brep_core::freeform_cuboid_solid([0., 0., 0.], [2., 2., 2.]).unwrap()).unwrap();
    let sexported = dispatch(json!({
        "op": "brep_nurbs_export_step_solid",
        "model": solid,
    }))
    .unwrap();
    let stext = sexported["text"].as_str().unwrap();
    assert!(stext.contains("MANIFOLD_SOLID_BREP"));
    assert!(!stext.contains("OSCAD_SOLID"));
    assert!(!stext.contains("AABB"));
    assert_eq!(
        sexported["certificate"]["capability"].as_str(),
        Some("nurbs-step-solid/1")
    );
    let simported = dispatch(json!({
        "op": "brep_nurbs_import_step_solid",
        "text": sexported["text"],
    }))
    .unwrap();
    assert_eq!(simported["model"]["faces"].as_array().unwrap().len(), 6);

    let outer = brep_core::freeform_cuboid_solid([0., 0., 0.], [4., 4., 4.]).unwrap();
    let inner = brep_core::freeform_cuboid_solid([2., -1., 0.], [5., 3., 4.]).unwrap();
    let (boolean_result, boolean_cert) =
        brep_core::nurbs_boolean_imprint_solids(&outer, &inner, "difference").unwrap();
    assert_eq!(boolean_cert.capability, "nurbs-boolean-bezier-le3/2");
    assert!(boolean_cert.permits_topology_change());
    boolean_result.validate().unwrap();
}
