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
fn direct_step_v3_bridge_preserves_exact_graph() {
    let model = brep_core::freeform_cuboid_solid([0., 0., 0.], [2., 3., 4.]).unwrap();
    let expected = encode(model).unwrap();
    let exported = dispatch(json!({
        "op": "brep_nurbs_export_step_v3",
        "model": expected,
    }))
    .unwrap();
    assert_eq!(
        exported["certificate"]["capability"].as_str(),
        Some("step-interchange/3")
    );
    assert_eq!(exported["identity"]["preserved"], true);
    let imported = dispatch(json!({
        "op": "brep_nurbs_import_step_v3",
        "text": exported["text"],
    }))
    .unwrap();
    assert_eq!(imported["identity"]["preserved"], true);
    assert_eq!(imported["model"]["vertices"].as_array().unwrap().len(), 8);
    assert_eq!(imported["model"]["edges"].as_array().unwrap().len(), 12);
    assert_eq!(imported["model"]["faces"].as_array().unwrap().len(), 6);
    assert_eq!(imported["model"]["bodies"].as_array().unwrap().len(), 1);
}

#[test]
fn direct_step_v5_bridge_exposes_ap242_report() {
    let model = brep_core::freeform_cuboid_solid([0., 0., 0.], [2., 3., 4.]).unwrap();
    let exported=dispatch(json!({"op":"brep_nurbs_export_step_v5","model":encode(model).unwrap()})).unwrap();
    assert_eq!(exported["certificate"]["capability"],"step-interchange/5");
    assert!(exported["text"].as_str().unwrap().contains("SHAPE_DEFINITION_REPRESENTATION"));
    let imported=dispatch(json!({"op":"brep_nurbs_import_step_v5","text":exported["text"]})).unwrap();
    assert_eq!(imported["identity"]["preserved"],true);
    assert_eq!(imported["metadataLoss"].as_array().unwrap().len(),0);
}

#[test]
fn direct_step_v6_bridge_roundtrips_poles_and_reports_identities(){
    let model=brep_core::frustum(2.,0.,3.).unwrap();
    let exported=dispatch(json!({"op":"brep_nurbs_export_step_v6","model":encode(model).unwrap()})).unwrap();
    assert_eq!(exported["certificate"]["capability"],"step-interchange/6");
    let imported=dispatch(json!({"op":"brep_nurbs_import_step_v6","text":exported["text"]})).unwrap();
    assert_eq!(imported["model"]["bodies"].as_array().unwrap().len(),1);
    assert_eq!(imported["definitionIdentities"].as_array().unwrap().len(),1);
    assert_eq!(imported["occurrenceIdentities"].as_array().unwrap().len(),1);
}

#[test]
fn direct_step_v7_bridge_composes_occurrence_topology(){
    let model=brep_core::freeform_cuboid_solid([0.,0.,0.],[1.,1.,1.]).unwrap();
    let exported=dispatch(json!({"op":"brep_nurbs_export_step_v7","model":encode(model).unwrap()})).unwrap();
    assert_eq!(exported["certificate"]["capability"],"step-interchange/7");
    let imported=dispatch(json!({"op":"brep_nurbs_import_step_v7","text":exported["text"]})).unwrap();
    let composed=dispatch(json!({"op":"brep_nurbs_compose_step_v7","models":[imported["model"].clone(),imported["model"].clone()]})).unwrap();
    assert_eq!(composed["bodies"].as_array().unwrap().len(),2);
    let ids=composed["topologyIds"]["bodies"].as_array().unwrap();
    assert_eq!(ids.len(),2);
    assert_ne!(ids[0],ids[1]);
}

#[test]
fn direct_step_v8_bridge_exposes_whole_domain_certificate(){
    let model=brep_core::sphere(2.).unwrap();
    let exported=dispatch(json!({"op":"brep_nurbs_export_step_v8","model":encode(model).unwrap()})).unwrap();
    assert_eq!(exported["certificate"]["capability"],"step-interchange/8");
    assert_eq!(exported["certificate"]["coupledSenseCases"],128);
    assert_eq!(exported["certificate"]["regularity"][0]["carrier"],"sphere");
    let imported=dispatch(json!({"op":"brep_nurbs_import_step_v8","text":exported["text"]})).unwrap();
    assert_eq!(imported["certificate"]["complete"],true);
    assert_eq!(imported["certificate"]["regularity"][0]["collapsedBoundaries"].as_array().unwrap().len(),2);
}

#[test]
fn direct_step_v9_bridge_retains_open_shell(){
    let control_points=(0..4).map(|u|(0..4).map(|v|
        vec![u as f64/3.,v as f64/3.,(u*v) as f64/90.]).collect()).collect();
    let surface=nurbs_core::surface::Surface{degree_u:3,degree_v:3,
        knots_u:vec![0.,0.,0.,0.,1.,1.,1.,1.],knots_v:vec![0.,0.,0.,0.,1.,1.,1.,1.],
        control_points,weights:vec![vec![1.;4];4],periodic_u:false,periodic_v:false};
    let model=brep_core::bicubic_open_face(surface).unwrap();
    let exported=dispatch(json!({"op":"brep_nurbs_export_step_v9","model":encode(model).unwrap()})).unwrap();
    assert_eq!(exported["certificate"]["capability"],"step-interchange/9");
    assert!(exported["text"].as_str().unwrap().contains("SHELL_BASED_SURFACE_MODEL"));
    let imported=dispatch(json!({"op":"brep_nurbs_import_step_v9","text":exported["text"]})).unwrap();
    assert_eq!(imported["model"]["bodies"].as_array().unwrap().len(),0);
    assert_eq!(imported["model"]["shells"][0]["closed"],false);
}

#[test]
fn direct_iges_v2_bridge_preserves_exact_graph() {
    let model = brep_core::freeform_cuboid_solid([0., 0., 0.], [2., 3., 4.]).unwrap();
    let exported = dispatch(json!({
        "op":"brep_nurbs_export_iges_v2",
        "model":encode(model).unwrap(),
    })).unwrap();
    assert_eq!(exported["certificate"]["capability"].as_str(),Some("iges-interchange/2"));
    assert_eq!(exported["identity"]["preserved"],true);
    assert!(exported["text"].as_str().unwrap().lines().all(|line|line.len()==80));
    let imported=dispatch(json!({"op":"brep_nurbs_import_iges_v2","text":exported["text"]})).unwrap();
    assert_eq!(imported["identity"]["preserved"],true);
    assert_eq!(imported["model"]["vertices"].as_array().unwrap().len(),8);
    assert_eq!(imported["model"]["edges"].as_array().unwrap().len(),12);
    assert_eq!(imported["model"]["faces"].as_array().unwrap().len(),6);
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
        "model":encode(model.clone()).unwrap(),
        "edges":edges.clone(),
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
    let exact_fillet = dispatch(json!({
        "op":"brep_nurbs_exact_convex_prism_fillet",
        "model":encode(model.clone()).unwrap(),
        "edges":edges.clone(),
        "radius":0.4
    }))
    .unwrap();
    assert_eq!(
        exact_fillet["certificate"]["capability"].as_str(),
        Some("exact-convex-prism-edge-fillet/1")
    );
    assert_eq!(exact_fillet["audit"]["ok"], true);
    let vertical = edges[0];
    let connected = model.edges[0]
        .vertices
        .iter()
        .find_map(|vertex| {
            (1..model.edges.len()).find(|edge| model.edges[*edge].vertices.contains(vertex))
        })
        .unwrap();
    let chamfer = dispatch(json!({
        "op":"brep_nurbs_exact_convex_chamfer",
        "model":encode(model.clone()).unwrap(),
        "edges":[0,connected],
        "distance":0.5
    }))
    .unwrap();
    assert_eq!(
        chamfer["certificate"]["capability"].as_str(),
        Some("exact-convex-straight-edge-chamfer/1")
    );
    assert_eq!(chamfer["audit"]["ok"], true);
    let variable = dispatch(json!({
        "op":"brep_nurbs_exact_variable_radius_fillet",
        "model":encode(model.clone()).unwrap(),
        "edges":[vertical],
        "radii":[[0.4,0.6]]
    }))
    .unwrap();
    assert_eq!(
        variable["certificate"]["capability"].as_str(),
        Some("exact-variable-radius-fillet/1")
    );
    assert_eq!(variable["audit"]["ok"], true);
    assert!(dispatch(json!({
        "op":"brep_nurbs_exact_variable_radius_fillet",
        "model":encode(model.clone()).unwrap(),
        "edges":[vertical],
        "radii":[[0.5,0.5]]
    }))
    .is_err());
    let max = [10., 8., 6.];
    let corner_edges: Vec<usize> = model
        .edges
        .iter()
        .enumerate()
        .filter_map(|(index, edge)| {
            let touches = edge.vertices.iter().any(|&v| {
                let p = model.vertices[v].point;
                (0..3).all(|i| (p[i] - max[i]).abs() <= 1e-9)
            });
            let a = model.vertices[edge.vertices[0]].point;
            let b = model.vertices[edge.vertices[1]].point;
            let mid = [
                0.5 * (a[0] + b[0]),
                0.5 * (a[1] + b[1]),
                0.5 * (a[2] + b[2]),
            ];
            let axis = ((mid[0] - max[0]).abs() <= 1e-9 && (mid[1] - max[1]).abs() <= 1e-9)
                || ((mid[0] - max[0]).abs() <= 1e-9 && (mid[2] - max[2]).abs() <= 1e-9)
                || ((mid[1] - max[1]).abs() <= 1e-9 && (mid[2] - max[2]).abs() <= 1e-9);
            (touches && axis).then_some(index)
        })
        .collect();
    assert_eq!(corner_edges.len(), 3);
    let valence3 = dispatch(json!({
        "op":"brep_nurbs_exact_valence3_corner_blend",
        "model":encode(model).unwrap(),
        "edges":corner_edges,
        "radius":1.0
    }))
    .unwrap();
    assert_eq!(
        valence3["certificate"]["capability"].as_str(),
        Some("exact-valence3-corner-blend/1")
    );
    assert_eq!(valence3["audit"]["ok"], true);

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
fn exact_analytic_shell_crosses_bridge_with_audited_certificate() {
    let model = brep_core::cuboid([0., 0., 0.], [10., 8., 6.]).unwrap();
    let before = value_codec::to_string(&model).unwrap();
    let result = dispatch(json!({
        "op":"brep_nurbs_exact_analytic_shell",
        "model":encode(model.clone()).unwrap(),
        "openings":[0,1],
        "thickness":0.5,
        "direction":"inward"
    }))
    .unwrap();
    assert_eq!(
        result["certificate"]["capability"].as_str(),
        Some("analytic-shell/2")
    );
    assert_eq!(result["certificate"]["complete"], true);
    assert_eq!(result["audit"]["ok"], true);
    assert_eq!(result["namingComplete"], true);
    assert!(result["changeSet"]["changes"].as_array().unwrap().len() > 0);
    assert_eq!(before, value_codec::to_string(&model).unwrap());
    assert!(dispatch(json!({
        "op":"brep_nurbs_exact_analytic_shell",
        "model":encode(model).unwrap(),
        "openings":[0,2],
        "thickness":0.5,
        "direction":"inward"
    }))
    .is_err());
}

#[test]
fn exact_loft_and_bent_rmf_successors_cross_bridge_atomically() {
    let loft = dispatch(json!({
        "op":"brep_nurbs_audited_multi_section_loft_v2",
        "sections":[
            [[-1.,-1.,0.],[1.,-1.,0.],[1.,1.,0.],[-1.,1.,0.]],
            [[-1.5,-1.,2.],[1.5,-1.,2.],[1.5,1.,2.],[-1.5,1.,2.]],
            [[-1.,-0.75,5.],[1.,-0.75,5.],[1.,0.75,5.],[-1.,0.75,5.]]
        ]
    }))
    .unwrap();
    assert_eq!(
        loft["certificate"]["capability"].as_str(),
        Some("analytic-solid-loft/2")
    );
    assert_eq!(loft["audit"]["ok"], true);
    assert_eq!(loft["model"]["faces"].as_array().unwrap().len(), 10);

    let sweep = dispatch(json!({
        "op":"brep_nurbs_audited_bent_rmf_sweep_v2",
        "profile":[[-0.2,-0.2],[0.2,-0.2],[0.2,0.2],[-0.2,0.2]],
        "path":[[0.,0.,0.],[0.,0.,3.],[0.,1.,6.],[0.,3.,9.]],
        "twistRadians":[0.,0.1,0.2,0.3],
        "scales":[1.,1.1,1.2,1.25]
    }))
    .unwrap();
    assert_eq!(
        sweep["certificate"]["capability"].as_str(),
        Some("exact-parallel-frame-sweep/2")
    );
    assert_eq!(sweep["audit"]["ok"], true);
    assert_eq!(sweep["namingComplete"], true);
    assert_eq!(sweep["model"]["faces"].as_array().unwrap().len(), 14);
    assert!(dispatch(json!({
        "op":"brep_nurbs_audited_bent_rmf_sweep_v2",
        "profile":[[-0.2,-0.2],[0.2,-0.2],[0.2,0.2],[-0.2,0.2]],
        "path":[[0.,0.,0.],[0.,0.,3.],[0.,0.,0.]],
        "twistRadians":[0.,0.,0.],
        "scales":[1.,1.,1.]
    }))
    .is_err());
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

#[test]
fn authorized_heal_v2_bridge_derives_evidence_and_preserves_source() {
    let model = brep_core::cuboid([0., 0., 0.], [2., 2., 2.]).unwrap();
    let vertex = model.edges[0].vertices[0];
    let incident = model
        .edges
        .iter()
        .filter(|edge| edge.vertices.contains(&vertex))
        .count();
    let mut to = model.vertices[vertex].point;
    to[1] += model.tolerance_context().unwrap().spatial_bounds().absolute_mm
        / (incident + 1) as f64
        * 0.25;
    let source = encode(model).unwrap();
    let before = source.clone();
    let result = dispatch(json!({
        "op":"brep_nurbs_authorized_heal_v2",
        "model":source,
        "operation":{"kind":"endpointSnap","vertex":vertex,"to":to}
    }))
    .unwrap();
    assert_eq!(source, before);
    assert_eq!(
        result["certificate"]["capability"].as_str(),
        Some("authorized-heal-gap-le1/2")
    );
    assert_eq!(result["certificate"]["status"].as_str(), Some("Complete"));
    assert_eq!(result["certificate"]["namingComplete"].as_bool(), Some(true));
    assert!(
        result["certificate"]["displacementLedger"][0]["actualMm"]
            .as_f64()
            .unwrap()
            > 0.
    );
    assert!(
        dispatch(json!({
            "op":"brep_nurbs_authorized_heal_v2",
            "model":source,
            "operation":{"kind":"endpointSnap","vertex":vertex,"to":to,"evidence":true}
        }))
        .unwrap_err()
        .message
        .contains("unauthorized fields")
    );
}

#[test]
fn curved_graph_boolean_v3_bridge_serializes_audited_authority() {
    let graph = dispatch(json!({
        "op":"brep_nurbs_canonical_graph_solid_v3",
        "degreeU":3,
        "degreeV":2
    }))
    .unwrap();
    let cutter = encode(brep_core::cuboid([0.5, -1., -1.], [2., 2., 3.]).unwrap()).unwrap();
    let result = dispatch(json!({
        "op":"brep_nurbs_boolean_bezier_le3_v3",
        "a":graph,
        "b":cutter,
        "operation":"intersection"
    }))
    .unwrap();
    assert_eq!(
        result["certificate"]["capability"].as_str(),
        Some("nurbs-boolean-bezier-le3/3")
    );
    assert_eq!(result["certificate"]["status"].as_str(), Some("Complete"));
    assert_eq!(result["certificate"]["axis"].as_str(), Some("U"));
    assert_eq!(result["certificate"]["tensorCells"].as_u64(), Some(3));
    assert_eq!(
        result["certificate"]["lineage"]["generatedIntersectionEdge"]
            .as_str()
            .map(str::is_empty),
        Some(false)
    );
    assert_eq!(result["certificate"]["audit"]["ok"].as_bool(), Some(true));
    assert_eq!(result["certificate"]["namingComplete"].as_bool(), Some(true));
    assert!(
        dispatch(json!({
            "op":"brep_nurbs_boolean_bezier_le3_v3",
            "a":graph,
            "b":cutter,
            "operation":"union"
        }))
        .is_err()
    );
}

#[test]
fn curved_graph_boolean_v4_v5_bridge_serializes_cell_specific_authority() {
    let graph = dispatch(json!({
        "op":"brep_nurbs_canonical_graph_solid_v3",
        "degreeU":3,
        "degreeV":3
    }))
    .unwrap();
    let unequal = encode(brep_core::cuboid([0.5, -2., -3.], [2., 3., 4.]).unwrap()).unwrap();
    let v4 = dispatch(json!({
        "op":"brep_nurbs_boolean_bezier_le3_v4_unequal",
        "a":graph,
        "b":unequal,
        "operation":"intersection"
    }))
    .unwrap();
    assert_eq!(
        v4["certificate"]["capability"].as_str(),
        Some("nurbs-boolean-bezier-le3/4")
    );

    let inner = encode(brep_core::cuboid([0.2; 3], [0.8; 3]).unwrap()).unwrap();
    let contained = dispatch(json!({
        "op":"brep_nurbs_boolean_bezier_le3_v4_containment",
        "graph":graph,
        "cutter":inner,
        "operation":"difference"
    }))
    .unwrap();
    assert_eq!(contained["certificate"]["cavityProof"].as_bool(), Some(true));
    assert_eq!(contained["certificate"]["audit"]["shellCount"].as_u64(), Some(2));

    let rational = dispatch(json!({
        "op":"brep_nurbs_canonical_rational_graph_solid_v5",
        "degreeU":3,
        "degreeV":3
    }))
    .unwrap();
    let v5 = dispatch(json!({
        "op":"brep_nurbs_boolean_bezier_le3_v5_rational",
        "a":rational,
        "b":unequal,
        "operation":"difference"
    }))
    .unwrap();
    assert_eq!(
        v5["certificate"]["capability"].as_str(),
        Some("nurbs-boolean-bezier-le3/5")
    );
    assert_eq!(
        v5["certificate"]["homogeneousRootProof"].as_bool(),
        Some(true)
    );
    assert!(v5["certificate"]["denominatorLowerBound"].as_f64().unwrap() >= 0.25);
    assert!(v5["certificate"]["weightConditionNumber"].as_f64().unwrap() <= 8.);
    assert!(v5["certificate"]["resourceBound"].as_u64().unwrap() <= 16);
}

#[test]
fn general_multispan_boolean_bridge_is_strict_and_audited() {
    for (u, v) in [(2, 1), (1, 2), (2, 2)] {
        let graph = dispatch(json!({
            "op":"brep_nurbs_canonical_multispan_graph_solid",
            "spansU":u,
            "spansV":v
        }))
        .unwrap();
        let cutter = encode(brep_core::cuboid([0.25, -1., -1.], [0.75, 2., 3.]).unwrap())
            .unwrap();
        for (operation, components, faces) in
            [("intersection", 1, 6), ("difference", 2, 12), ("union", 3, 18)]
        {
            let result = dispatch(json!({
                "op":"brep_nurbs_boolean_general",
                "a":graph,
                "b":cutter,
                "operation":operation
            }))
            .unwrap();
            assert_eq!(
                result["certificate"]["capability"].as_str(),
                Some("nurbs-boolean/1")
            );
            assert_eq!(
                result["certificate"]["authority"].as_str(),
                Some("author-general-nurbs-boolean")
            );
            assert_eq!(result["certificate"]["status"].as_str(), Some("Complete"));
            assert_eq!(result["certificate"]["operandOrder"].as_str(), Some("source-tool"));
            assert_eq!(result["certificate"]["exactRegionMembership"].as_bool(), Some(true));
            assert_eq!(result["certificate"]["ssReportsComplete"].as_bool(), Some(true));
            assert!(result["certificate"]["ssFacePairs"].as_u64().unwrap_or(0) > 0);
            assert_eq!(result["certificate"]["branchGraph"]["components"].as_u64(), Some(2));
            assert_eq!(result["certificate"]["branchGraph"]["complete"].as_bool(), Some(true));
            assert_eq!(result["certificate"]["uv"]["complete"].as_bool(), Some(true));
            assert_eq!(result["certificate"]["resultComponents"].as_u64(), Some(components));
            assert_eq!(result["certificate"]["resultFaces"].as_u64(), Some(faces));
            assert_eq!(result["certificate"]["audit"]["ok"].as_bool(), Some(true));
            assert_eq!(result["certificate"]["noFallback"].as_bool(), Some(true));
        }
        let reversed = dispatch(json!({
            "op":"brep_nurbs_boolean_general",
            "a":cutter,
            "b":graph,
            "operation":"difference"
        }))
        .unwrap();
        assert_eq!(reversed["certificate"]["operandOrder"].as_str(), Some("tool-source"));
        assert_eq!(reversed["certificate"]["partitionCells"].as_u64(), Some(4));
        assert!(dispatch(json!({
            "op":"brep_nurbs_boolean_general",
            "a":graph,
            "b":cutter,
            "operation":"intersection",
            "fallback":"mesh"
        }))
        .is_err());
    }
}
