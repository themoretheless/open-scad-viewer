use super::*;
use brep_core::analytic::{cylinder, frustum, sphere, torus, tube};
use brep_core::cuboid;

fn envelope_ok(a: &Model, b: &Model, tol: f64) {
    let (amin, amax) = model_bounds(a);
    let (bmin, bmax) = model_bounds(b);
    for i in 0..3 {
        assert!((amin[i] - bmin[i]).abs() < tol, "min[{i}]");
        assert!((amax[i] - bmax[i]).abs() < tol, "max[{i}]");
    }
}

fn assert_graph_only(text: &str) {
    assert!(!text.contains("OSCAD_SOLID"));
    assert!(!text.contains("cuboid|"));
    assert!(!text.contains("cylinder|"));
    assert!(!text.contains("tube|"));
}

fn strip_identity_metadata(text: &str) -> String {
    text.lines()
        .map(|line| {
            if let Some(start) = line.find("'OSCAD_TOPO/2|") {
                let end = line[start + 1..].find('\'').unwrap() + start + 1;
                format!("{}''{}", &line[..start], &line[end + 1..])
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
        + "\n"
}

#[test]
fn successor_identity_units_and_metadata_loss() {
    let model = cuboid([0., 0., 0.], [3., 2., 1.]).unwrap();
    let original_ids = model.1.vertices.clone();
    let (text, cert, exported_identity) = export_step_v2(&model).unwrap();
    assert_eq!(cert.capability, STEP_INTERCHANGE_V2_CAPABILITY);
    assert!(text.contains("GLOBAL_UNIT_ASSIGNED_CONTEXT"));
    assert!(exported_identity.preserved);
    let (back, _, identity) = import_step_v2(&text).unwrap();
    assert!(identity.preserved);
    assert_eq!(back.1.vertices, original_ids);

    let stripped = strip_identity_metadata(&text);
    let (external, _, loss) = import_step_v2(&stripped).unwrap();
    assert!(!loss.preserved);
    assert_eq!(loss.created_count, topology_count(&external));
    assert_eq!(loss.lost_count, loss.created_count);

    let metres = stripped.replace(".MILLI.,.METRE.", "$,.METRE.");
    let (converted, _, _) = import_step_v2(&metres).unwrap();
    let (_, max) = model_bounds(&converted);
    assert!((max[0] - 3000.).abs() < 1e-7);
}

#[test]
fn successor_reordered_entities_duplicate_identity_and_nested_placement() {
    let model = cuboid([0., 0., 0.], [3., 2., 1.]).unwrap();
    let (text, _, _) = export_step_v2(&model).unwrap();
    let duplicate = text.replacen(
        &model.1.vertices[1].to_string(),
        &model.1.vertices[0].to_string(),
        1,
    );
    assert!(import_step_v2(&duplicate).is_err());

    let mut entities: Vec<_> = text.lines().filter(|line| line.starts_with('#')).collect();
    entities.reverse();
    let reordered = text
        .lines()
        .filter(|line| !line.starts_with('#'))
        .chain(entities)
        .collect::<Vec<_>>()
        .join("\n")
        + "\n";
    assert!(import_step_v2(&reordered).is_ok());

    let rows = "\
#800000001=CARTESIAN_POINT('',(0.,0.,0.));
#800000002=DIRECTION('',(0.,0.,1.));
#800000003=DIRECTION('',(1.,0.,0.));
#800000004=AXIS2_PLACEMENT_3D('',#800000001,#800000002,#800000003);
#800000005=CARTESIAN_POINT('',(1.,0.,0.));
#800000006=AXIS2_PLACEMENT_3D('',#800000005,#800000002,#800000003);
#800000007=CARTESIAN_POINT('',(0.,2.,0.));
#800000008=AXIS2_PLACEMENT_3D('',#800000007,#800000002,#800000003);
#800000009=ITEM_DEFINED_TRANSFORMATION('','',#800000004,#800000006);
#800000010=ITEM_DEFINED_TRANSFORMATION('','',#800000004,#800000008);
";
    let placed = text.replacen(
        "ENDSEC;\nEND-ISO-10303-21;",
        &format!("{rows}ENDSEC;\nEND-ISO-10303-21;"),
        1,
    );
    let (placed, _, _) = import_step_v2(&placed).unwrap();
    let (min, _) = model_bounds(&placed);
    assert!((min[0] - 1.).abs() < 1e-7);
    assert!((min[1] - 2.).abs() < 1e-7);
}

#[test]
fn successor_refuses_arbitrary_graph_entities() {
    let model = cuboid([0., 0., 0.], [1., 1., 1.]).unwrap();
    let (text, _, _) = export_step_v2(&model).unwrap();
    let arbitrary = text.replacen(
        "ENDSEC;\nEND-ISO-10303-21;",
        "#700000000=BLOCK('third-party',1.,1.,1.);\nENDSEC;\nEND-ISO-10303-21;",
        1,
    );
    let error = import_step_v2(&arbitrary).unwrap_err();
    assert_eq!(error.code, "BREP_STEP_REFUSED");
    assert!(error.message.contains("finite subset"));
}

#[test]
fn roundtrip_cuboid() {
    let model = cuboid([0., 0., 0.], [3., 2., 1.]).unwrap();
    let (text, cert) = export_step(&model).unwrap();
    assert!(cert.complete);
    assert!(text.contains("ADVANCED_FACE"));
    assert!(text.contains("PLANE"));
    assert!(text.contains("AXIS2_PLACEMENT_3D"));
    assert!(text.contains("VERTEX_POINT"));
    assert!(text.contains("EDGE_CURVE"));
    assert!(text.contains("EDGE_LOOP"));
    assert!(text.contains("AP242"));
    assert!(!text.contains("FACETED_BREP"));
    assert_graph_only(&text);
    assert_eq!(text.matches("EDGE_CURVE").count(), 12);
    assert_eq!(text.matches("VERTEX_POINT").count(), 8);
    let (back, icert) = import_step(&text).unwrap();
    assert!(icert.notes.contains(&"step_graph_only_import"));
    assert!(icert.notes.contains(&"oscad_solid_removed"));
    back.validate().unwrap();
    assert_eq!(back.faces.len(), model.faces.len());
    envelope_ok(&model, &back, 1e-9);
}

#[test]
fn roundtrip_cylinder_not_aabb() {
    let model = cylinder(2., 4.).unwrap();
    let (text, _) = export_step(&model).unwrap();
    assert!(text.contains("CYLINDRICAL_SURFACE"));
    assert!(text.contains("CIRCLE"));
    assert_graph_only(&text);
    let (back, _) = import_step(&text).unwrap();
    back.validate().unwrap();
    assert_eq!(back.faces.len(), model.faces.len());
    assert!(recognize_cylinder(&back).unwrap().is_some());
    assert!(
        back.faces
            .iter()
            .any(|f| f.surface.degree_u > 1 || f.surface.degree_v > 1)
    );
    envelope_ok(&model, &back, 1e-6);
}

#[test]
fn roundtrip_sphere() {
    let model = sphere(3.).unwrap();
    let (text, _) = export_step(&model).unwrap();
    assert!(text.contains("SPHERICAL_SURFACE"));
    assert!(text.contains("CIRCLE"));
    assert_graph_only(&text);
    let (back, _) = import_step(&text).unwrap();
    assert_eq!(back.faces.len(), 8);
    assert!(recognize_sphere(&back).unwrap().is_some());
    envelope_ok(&model, &back, 1e-6);
}

#[test]
fn roundtrip_frustum() {
    let model = frustum(3., 1., 5.).unwrap();
    let (text, _) = export_step(&model).unwrap();
    assert!(text.contains("CONICAL_SURFACE"));
    assert!(text.contains("CIRCLE"));
    assert_graph_only(&text);
    let (back, _) = import_step(&text).unwrap();
    assert!(recognize_cone(&back).unwrap().is_some());
    envelope_ok(&model, &back, 1e-5);
}

#[test]
fn roundtrip_torus() {
    let model = torus(4., 1.).unwrap();
    let (text, _) = export_step(&model).unwrap();
    assert!(text.contains("TOROIDAL_SURFACE"));
    assert!(text.contains("CIRCLE"));
    assert_graph_only(&text);
    let (back, _) = import_step(&text).unwrap();
    assert_eq!(back.faces.len(), 16);
    assert!(recognize_torus(&back).unwrap().is_some());
    envelope_ok(&model, &back, 1e-5);
}

#[test]
fn roundtrip_tube() {
    let model = tube(3., 1.5, 4.).unwrap();
    let (text, _) = export_step(&model).unwrap();
    assert!(text.contains("CYLINDRICAL_SURFACE"));
    assert!(text.contains("CIRCLE"));
    assert!(text.contains("FACE_BOUND"));
    assert!(text.contains("FACE_OUTER_BOUND"));
    assert_graph_only(&text);
    let (back, _) = import_step(&text).unwrap();
    assert_eq!(back.faces.len(), model.faces.len());
    envelope_ok(&model, &back, 1e-5);
}

#[test]
fn roundtrip_apex_frustum() {
    for model in [frustum(0., 3., 5.).unwrap(), frustum(3., 0., 5.).unwrap()] {
        let (text, _) = export_step(&model).unwrap();
        assert!(text.contains("CONICAL_SURFACE"));
        // Apex end has no CIRCLE; the nonzero ring still has one.
        assert!(text.contains("CIRCLE"));
        assert_graph_only(&text);
        let (back, _) = import_step(&text).unwrap();
        assert!(recognize_cone(&back).unwrap().is_some());
        envelope_ok(&model, &back, 1e-4);
    }
}

#[test]
fn roundtrip_oriented_tube() {
    let base = tube(3., 1.5, 4.).unwrap();
    let model = brep_core::transform::affine(
        &base,
        [
            [0., 0., 1., 1.],
            [1., 0., 0., 2.],
            [0., 1., 0., 3.],
            [0., 0., 0., 1.],
        ],
    )
    .unwrap();
    let (text, _) = export_step(&model).unwrap();
    assert!(text.contains("FACE_BOUND"));
    let (back, _) = import_step(&text).unwrap();
    assert_eq!(back.faces.len(), model.faces.len());
    envelope_ok(&model, &back, 1e-4);
}

#[test]
fn refuse_curved_without_circle() {
    let text = r#"ISO-10303-21;
HEADER;
FILE_SCHEMA(('AUTOMOTIVE_DESIGN','AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'));
ENDSEC;
DATA;
#1=CARTESIAN_POINT('',(0.,0.,0.));
#2=DIRECTION('',(0.,0.,1.));
#3=DIRECTION('',(1.,0.,0.));
#4=AXIS2_PLACEMENT_3D('',#1,#2,#3);
#5=CYLINDRICAL_SURFACE('',#4,2.);
#6=CARTESIAN_POINT('',(0.,0.,4.));
#7=AXIS2_PLACEMENT_3D('',#6,#2,#3);
#8=PLANE('',#4);
#9=PLANE('',#7);
#10=ADVANCED_FACE('',(#11),#5,.T.);
#11=FACE_OUTER_BOUND('',#12,.T.);
#12=EDGE_LOOP('',());
#13=ADVANCED_FACE('',(#14),#8,.T.);
#14=FACE_OUTER_BOUND('',#15,.T.);
#15=EDGE_LOOP('',());
#16=ADVANCED_FACE('',(#17),#9,.T.);
#17=FACE_OUTER_BOUND('',#18,.T.);
#18=EDGE_LOOP('',());
#19=CLOSED_SHELL('',(#10,#13,#16));
#20=MANIFOLD_SOLID_BREP('body',#19);
ENDSEC;
END-ISO-10303-21;
"#;
    assert_eq!(import_step(text).unwrap_err().code, "BREP_STEP_REFUSED");
}

#[test]
fn orphan_or_comment_circle_cannot_satisfy_curved_graph_gate() {
    let (text, _) = export_step(&cylinder(2., 4.).unwrap()).unwrap();
    let without_linked_circles = text.replace("CIRCLE(", "LINE(");
    let with_comment_only = format!("/* CIRCLE */\n{without_linked_circles}");
    let error = import_step(&with_comment_only).unwrap_err();
    assert_eq!(error.code, "BREP_STEP_REFUSED");
    assert!(error.message.contains("CIRCLE ring edges"));

    let with_orphan = without_linked_circles.replace(
        "ENDSEC;\nEND-ISO-10303-21;",
        "#999999=CIRCLE('',#1,2.);\nENDSEC;\nEND-ISO-10303-21;",
    );
    let error = import_step(&with_orphan).unwrap_err();
    assert_eq!(error.code, "BREP_STEP_REFUSED");
    assert!(error.message.contains("CIRCLE ring edges"));
}

#[test]
fn refuse_faceted_stl_obj_missing_manifold() {
    assert_eq!(
        import_step("ISO-10303-21;\nDATA;\n#1=FACETED_BREP('',#2);\nENDSEC;\n")
            .unwrap_err()
            .code,
        "BREP_STEP_REFUSED"
    );
    assert_eq!(
        import_step("solid ascii\nfacet normal 0 0 1\nendsolid\n")
            .unwrap_err()
            .code,
        "BREP_STEP_REFUSED"
    );
    assert_eq!(
        import_step("mtllib x.mtl\nv 0 0 0\nf 1 1 1\n")
            .unwrap_err()
            .code,
        "BREP_STEP_REFUSED"
    );
    assert_eq!(
        import_step("ISO-10303-21;\nDATA;\n#1=ADVANCED_FACE('',(#2),#3,.T.);\nENDSEC;\n")
            .unwrap_err()
            .code,
        "BREP_STEP_REFUSED"
    );
}

#[test]
fn refuse_broken_refs() {
    let text = r#"ISO-10303-21;
HEADER;
FILE_SCHEMA(('AUTOMOTIVE_DESIGN'));
ENDSEC;
DATA;
#1=MANIFOLD_SOLID_BREP('body',#2);
#2=CLOSED_SHELL('',(#3));
#3=ADVANCED_FACE('',(#4),#999,.T.);
ENDSEC;
END-ISO-10303-21;
"#;
    assert_eq!(import_step(text).unwrap_err().code, "BREP_STEP_REFUSED");
}

#[test]
fn refuse_cylinder_without_plane_span() {
    let text = r#"ISO-10303-21;
HEADER;
FILE_SCHEMA(('AUTOMOTIVE_DESIGN','AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'));
ENDSEC;
DATA;
#1=CARTESIAN_POINT('',(0.,0.,0.));
#2=DIRECTION('',(0.,0.,1.));
#3=DIRECTION('',(1.,0.,0.));
#4=AXIS2_PLACEMENT_3D('',#1,#2,#3);
#5=CYLINDRICAL_SURFACE('',#4,2.);
#6=ADVANCED_FACE('',(#7),#5,.T.);
#7=FACE_OUTER_BOUND('',#8,.T.);
#8=EDGE_LOOP('',());
#9=CLOSED_SHELL('',(#6));
#10=MANIFOLD_SOLID_BREP('body',#9);
ENDSEC;
END-ISO-10303-21;
"#;
    assert_eq!(import_step(text).unwrap_err().code, "BREP_STEP_REFUSED");
}

#[test]
fn refuse_cone_without_plane_span() {
    let text = r#"ISO-10303-21;
HEADER;
FILE_SCHEMA(('AUTOMOTIVE_DESIGN','AP242_MANAGED_MODEL_BASED_3D_ENGINEERING'));
ENDSEC;
DATA;
#1=CARTESIAN_POINT('',(0.,0.,0.));
#2=DIRECTION('',(0.,0.,1.));
#3=DIRECTION('',(1.,0.,0.));
#4=AXIS2_PLACEMENT_3D('',#1,#2,#3);
#5=CONICAL_SURFACE('',#4,3.,0.4);
#6=ADVANCED_FACE('',(#7),#5,.T.);
#7=FACE_OUTER_BOUND('',#8,.T.);
#8=EDGE_LOOP('',());
#9=CLOSED_SHELL('',(#6));
#10=MANIFOLD_SOLID_BREP('body',#9);
ENDSEC;
END-ISO-10303-21;
"#;
    assert_eq!(import_step(text).unwrap_err().code, "BREP_STEP_REFUSED");
}
