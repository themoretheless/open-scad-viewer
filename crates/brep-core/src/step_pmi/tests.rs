use super::*;
use brep_topology::TopoKind;

fn face(label: &str) -> TopoId {
    TopoId::derive(
        TopoKind::Face,
        "step-pmi-test",
        label,
        "face",
        label.as_bytes(),
    )
}

fn semantic_fixture() -> String {
    r#"ISO-10303-21;
DATA;
#1=PRODUCT_DEFINITION_SHAPE('','',#2);
#2=PRODUCT_DEFINITION('','',#3,#4);
#3=PRODUCT_DEFINITION_FORMATION('','',#5);
#4=PRODUCT_DEFINITION_CONTEXT('part',#6,'design');
#5=PRODUCT('p','p','',(#6));
#6=PRODUCT_CONTEXT('',#7,'mechanical');
#7=APPLICATION_CONTEXT('managed model based 3d engineering');
#10=ADVANCED_FACE('',(),#999,.T.);
#11=ADVANCED_FACE('',(),#999,.T.);
#20=SHAPE_ASPECT('position face','',#1,.T.);
#21=GEOMETRIC_ITEM_SPECIFIC_USAGE('face usage','',#1,(#20),(#10));
#30=DATUM('D1','',#1,.F.,'A');
#31=GEOMETRIC_ITEM_SPECIFIC_USAGE('datum usage','',#1,(#30),(#11));
#40=DATUM_REFERENCE_COMPARTMENT('',$,#1,.F.,#30,(SIMPLE_DATUM_REFERENCE_MODIFIER(.MAXIMUM_MATERIAL_REQUIREMENT.)));
#41=DATUM_SYSTEM('DS1','',#1,.F.,(#40));
#42=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.25),#50);
#50=SI_UNIT(.MILLI.,.METRE.);
#60=(GEOMETRIC_TOLERANCE('position','',#42,#20)
 GEOMETRIC_TOLERANCE_WITH_DATUM_REFERENCE((#41))
 GEOMETRIC_TOLERANCE_WITH_MODIFIERS(.MAXIMUM_MATERIAL_REQUIREMENT.)
 POSITION_TOLERANCE());
#999=PLANE('',#998);
#998=AXIS2_PLACEMENT_3D('',#997,$,$);
#997=CARTESIAN_POINT('',(0.,0.,0.));
ENDSEC;
END-ISO-10303-21;"#.into()
}

#[test]
fn imports_complex_semantics_modifiers_and_persistent_faces() {
    let model = import_step_pmi(
        &semantic_fixture(),
        &BTreeMap::from([(10, face("10")), (11, face("11"))]),
    )
    .unwrap();
    assert_eq!(model.tolerances.len(), 1);
    let tolerance = &model.tolerances[0];
    assert_eq!(tolerance.kind, GeometricToleranceKind::Position);
    assert_eq!(tolerance.magnitude, 0.25);
    assert_eq!(tolerance.face_id, face("10"));
    assert_eq!(
        tolerance.modifiers,
        vec![PmiModifier::MaximumMaterialRequirement]
    );
    assert_eq!(
        tolerance.datum_references[0].precedence,
        DatumPrecedence::Primary
    );
    assert_eq!(tolerance.datum_references[0].datums[0].label, "A");
    assert_eq!(tolerance.datum_references[0].datums[0].face_id, face("11"));
    assert_eq!(
        tolerance.datum_references[0].modifiers,
        vec![PmiModifier::MaximumMaterialRequirement]
    );
    assert_eq!(model.shape_aspect_to_face.get(&20), Some(&face("10")));
    let json = model.to_json().unwrap();
    assert!(json.contains("\"kind\":\"POSITION_TOLERANCE\""));
    assert!(json.contains("\"magnitude\":2.50000000000000000E-1"));
    assert_eq!(json, model.to_json().unwrap());
}

#[test]
fn follows_shape_aspect_relationship_for_face_binding() {
    let source = r#"#1=SHAPE_ASPECT('composite','',#90,.T.);
#2=SHAPE_ASPECT('member','',#90,.T.);
#3=SHAPE_ASPECT_RELATIONSHIP('','',#1,#2);
#4=GEOMETRIC_ITEM_SPECIFIC_USAGE('','',#90,(#2),(#10));
#5=LENGTH_MEASURE_WITH_UNIT(LENGTH_MEASURE(0.1),#6);
#6=SI_UNIT(.MILLI.,.METRE.);
#7=FLATNESS_TOLERANCE('flat','',#5,#1);
#10=ADVANCED_FACE('',(),#11,.T.);
#11=PLANE('',#12);
#12=AXIS2_PLACEMENT_3D('',#13,$,$);
#13=CARTESIAN_POINT('',(0.,0.,0.));
#90=PRODUCT_DEFINITION_SHAPE('','',#91);
#91=PRODUCT_DEFINITION('','',#92,#93);
#92=PRODUCT_DEFINITION_FORMATION('','',#94);
#93=PRODUCT_DEFINITION_CONTEXT('',#95,'');
#94=PRODUCT('','','',(#95));
#95=PRODUCT_CONTEXT('',#96,'');
#96=APPLICATION_CONTEXT('');"#;
    let model = import_step_pmi(source, &BTreeMap::from([(10, face("10"))])).unwrap();
    assert_eq!(model.tolerances[0].face_id, face("10"));
    assert_eq!(model.shape_aspects[0].related_aspects, vec![2]);
}

#[test]
fn presentation_only_is_explicit_and_polylines_survive() {
    let source = r#"#1=ANNOTATION_CURVE_OCCURRENCE('picture','',#2);
#2=POLYLINE('',(#3,#4));
#3=CARTESIAN_POINT('',(0.,0.,0.));
#4=CARTESIAN_POINT('',(1.,2.,3.));"#;
    let model = import_step_pmi(source, &BTreeMap::new()).unwrap();
    assert!(model.is_presentation_only());
    assert_eq!(model.presentation_polylines[0].points[1], [1., 2., 3.]);
    assert!(
        model
            .diagnostics
            .iter()
            .any(|d| d.kind == PmiDiagnosticKind::PresentationOnly)
    );
    assert!(model.tolerances.is_empty());
}

#[test]
fn tessellated_picture_without_lines_still_has_honest_diagnostic() {
    let model = import_step_pmi(
        "#1=TESSELLATED_ANNOTATION_OCCURRENCE('picture','',#2);\n#2=TESSELLATED_PRESENTATION_ITEM('');",
        &BTreeMap::new(),
    )
    .unwrap();
    assert!(model.is_presentation_only());
    assert!(
        model
            .diagnostics
            .iter()
            .any(|diagnostic| { diagnostic.kind == PmiDiagnosticKind::PresentationOnly })
    );
    assert!(
        model.diagnostics.iter().any(|diagnostic| {
            diagnostic.kind == PmiDiagnosticKind::UnsupportedPresentation
        })
    );
}

#[test]
fn dangling_semantic_face_is_typed_error() {
    let mut source = semantic_fixture();
    source = source.replace("(#20),(#10)", "(#20),(#9999)");
    let error = import_step_pmi(
        &source,
        &BTreeMap::from([(10, face("10")), (11, face("11"))]),
    )
    .unwrap_err();
    assert_eq!(error.kind, PmiErrorKind::DanglingFace);
    assert_eq!(error.entity, Some(60));
}

#[test]
fn presentation_export_is_byte_identical_and_reimports_as_picture() {
    let model = import_step_pmi(
        &semantic_fixture(),
        &BTreeMap::from([(10, face("10")), (11, face("11"))]),
    )
    .unwrap();
    let layouts = BTreeMap::from([
        (
            30,
            PmiPresentationLayout {
                anchor: [1., 0., 0.],
                frame_origin: [2., 3., 0.],
            },
        ),
        (
            60,
            PmiPresentationLayout {
                anchor: [0., 0., 0.],
                frame_origin: [4., 5., 0.],
            },
        ),
    ]);
    let first = export_step_pmi_presentation(&model, &layouts, 2.).unwrap();
    let second = export_step_pmi_presentation(&model, &layouts, 2.).unwrap();
    assert_eq!(first.part21, second.part21);
    assert_eq!(first.polylines, second.polylines);
    assert!(
        first
            .polylines
            .iter()
            .any(|line| line.role == PmiPolylineRole::FeatureControlFrame)
    );
    assert!(
        first
            .polylines
            .iter()
            .any(|line| line.role == PmiPolylineRole::Text)
    );
    let picture = import_step_pmi(&first.part21, &BTreeMap::new()).unwrap();
    assert!(picture.is_presentation_only());
    assert_eq!(picture.presentation_polylines.len(), first.polylines.len());
    assert_eq!(
        picture.presentation_polylines[0].points,
        first.polylines[0].points
    );
}

#[test]
fn missing_layout_and_bad_modifier_are_not_silent() {
    let model = import_step_pmi(
        &semantic_fixture(),
        &BTreeMap::from([(10, face("10")), (11, face("11"))]),
    )
    .unwrap();
    let error = export_step_pmi_presentation(&model, &BTreeMap::new(), 1.).unwrap_err();
    assert_eq!(error.kind, PmiErrorKind::MissingLayout);
    let source = semantic_fixture().replace(
        ".MAXIMUM_MATERIAL_REQUIREMENT.)\n POSITION_TOLERANCE",
        ".NOT_A_MODIFIER.)\n POSITION_TOLERANCE",
    );
    let error = import_step_pmi(
        &source,
        &BTreeMap::from([(10, face("10")), (11, face("11"))]),
    )
    .unwrap();
    assert_eq!(
        error.tolerances[0].modifiers,
        vec![PmiModifier::OtherStandard("NOT_A_MODIFIER".into())]
    );
    assert!(model.to_json().unwrap().contains("\"M\""));
}

#[test]
fn composite_datum_members_keep_order_and_shared_precedence() {
    let source = semantic_fixture()
        .replace("#30=DATUM('D1','',#1,.F.,'A');", "#30=DATUM('D1','',#1,.F.,'A');\n#32=DATUM('D2','',#1,.F.,'B');\n#33=GEOMETRIC_ITEM_SPECIFIC_USAGE('datum usage','',#1,(#32),(#11));")
        .replace("#40=DATUM_REFERENCE_COMPARTMENT('',$,#1,.F.,#30,", "#40=DATUM_REFERENCE_COMPARTMENT('',$,#1,.F.,(#30,#32),");
    let model = import_step_pmi(
        &source,
        &BTreeMap::from([(10, face("10")), (11, face("11"))]),
    )
    .unwrap();
    let reference = &model.tolerances[0].datum_references[0];
    assert_eq!(
        reference
            .datums
            .iter()
            .map(|d| d.label.as_str())
            .collect::<Vec<_>>(),
        vec!["A", "B"]
    );
    assert_eq!(reference.precedence, DatumPrecedence::Primary);
}
