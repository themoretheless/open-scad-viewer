use super::*;
use crate::step_interchange_v3::step_xde::{
    StepValidationKind, StepValidationProperties, decode_step_string, encode_step_string,
};
use std::collections::BTreeMap;

fn colour(red: f64, green: f64, blue: f64) -> StepPresentationStyle {
    StepPresentationStyle {
        colour: Some(StepColour::new(red, green, blue).unwrap()),
        layers: Vec::new(),
    }
}

#[test]
fn presentation_priority_is_face_then_body_then_product() {
    let model = brep_core::cuboid([0.; 3], [2., 3., 4.]).unwrap();
    let mut semantic = StepSemanticMetadata::empty(&model);
    semantic.presentation.product_styles.insert(7, colour(0., 0., 1.));
    semantic.presentation.body_products[0] = Some(7);
    assert_eq!(
        semantic.presentation.face_style(0, 0).unwrap().colour,
        semantic.presentation.product_styles[&7].colour
    );
    semantic.presentation.body_styles[0] = Some(colour(0., 1., 0.));
    assert_eq!(
        semantic.presentation.face_style(0, 0).unwrap().colour,
        semantic.presentation.body_styles[0].as_ref().unwrap().colour
    );
    semantic.presentation.face_styles[0] = Some(colour(1., 0., 0.));
    assert_eq!(
        semantic.presentation.face_style(0, 0).unwrap().colour,
        Some(StepColour::new(1., 0., 0.).unwrap())
    );
    assert_eq!(
        semantic.presentation.face_style(1, 0).unwrap().colour,
        Some(StepColour::new(0., 1., 0.).unwrap())
    );
}

#[test]
fn presentation_xde_round_trip_and_validation_diagnostics() {
    let model = brep_core::cuboid([0.; 3], [2., 3., 4.]).unwrap();
    let mut semantic = StepSemanticMetadata::empty(&model);
    semantic.xde.products.insert(42, StepProductMetadata {
        entity: 42,
        id: "part-42".into(),
        name: "Корпус δ".into(),
        description: "validation fixture".into(),
        shape_name: Some("Основная форма".into()),
        material: Some("Сталь 09Г2С".into()),
        density_kg_per_m3: Some(7850.),
        validation: StepValidationProperties {
            volume_mm3: Some(24.),
            surface_area_mm2: Some(52.),
        },
        ..StepProductMetadata::default()
    });
    semantic.xde.body_products[0] = Some(42);
    semantic.presentation.body_products[0] = Some(42);
    semantic.presentation.product_styles.insert(42, StepPresentationStyle {
        layers: vec!["Product layer".into()],
        ..colour(0., 0., 1.)
    });
    semantic.presentation.body_styles[0] = Some(StepPresentationStyle {
        layers: vec!["Body layer".into()],
        ..colour(0., 1., 0.)
    });
    semantic.presentation.face_styles[0] = Some(StepPresentationStyle {
        layers: vec!["Face layer".into()],
        ..colour(1., 0., 0.)
    });

    let (step, _, _) = export_step_v9_with_metadata(&model, &semantic).unwrap();
    assert!(step.contains("STYLED_ITEM"));
    assert!(step.contains("PRESENTATION_LAYER_ASSIGNMENT"));
    assert!(step.contains("FILL_AREA_STYLE_COLOUR"));
    assert!(step.contains("SHAPE_DEFINITION_REPRESENTATION"));
    assert!(step.contains("MATERIAL_DESIGNATION"));
    assert!(step.contains("DENSITY_MEASURE"));
    assert!(step.contains("VOLUME_MEASURE"));

    let (_, _, _, imported) = import_step_v9_with_metadata(&step).unwrap();
    let product = imported.xde.products.values().next().unwrap();
    assert_eq!(product.id, "part-42");
    assert_eq!(product.name, "Корпус δ");
    assert_eq!(product.shape_name.as_deref(), Some("Основная форма"));
    assert_eq!(product.material.as_deref(), Some("Сталь 09Г2С"));
    assert!((product.density_kg_per_m3.unwrap() - 7850.).abs() < 1e-8);
    assert!((product.validation.volume_mm3.unwrap() - 24.).abs() < 1e-7);
    assert!((product.validation.surface_area_mm2.unwrap() - 52.).abs() < 1e-7);
    assert_eq!(
        imported.presentation.face_style(0, 0).unwrap().colour,
        Some(StepColour::new(1., 0., 0.).unwrap())
    );
    assert_eq!(
        imported.presentation.face_style(1, 0).unwrap().colour,
        Some(StepColour::new(0., 1., 0.).unwrap())
    );
    assert_eq!(
        imported.presentation.face_style(0, 0).unwrap().layers,
        vec!["Face layer"]
    );
    assert!(imported
        .xde
        .validation_diagnostics(&model, 1e-7, 1e-6)
        .unwrap()
        .is_empty());

    let (_, _, _, mut mismatch) = import_step_v9_with_metadata(&step).unwrap();
    let product = mismatch.xde.products.values_mut().next().unwrap();
    product.validation.volume_mm3 = Some(25.);
    product.validation.surface_area_mm2 = Some(50.);
    let diagnostics = mismatch
        .xde
        .validation_diagnostics(&model, 1e-7, 1e-6)
        .unwrap();
    assert_eq!(diagnostics.len(), 2);
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.property == StepValidationKind::Volume
            && (diagnostic.absolute_error - 1.).abs() < 1e-6
    }));
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.property == StepValidationKind::SurfaceArea
            && (diagnostic.absolute_error - 2.).abs() < 1e-6
    }));

    let (second, _, _) = export_step_v9_with_metadata(&model, &imported).unwrap();
    let (_, _, _, second) = import_step_v9_with_metadata(&second).unwrap();
    assert_eq!(second.xde.products.values().next().unwrap().name, "Корпус δ");
    assert_eq!(
        second.xde.products.values().next().unwrap().density_kg_per_m3,
        Some(7850.)
    );
}

#[test]
fn part21_name_escapes_are_symmetric() {
    let name = "Корпус δ 'A'";
    let encoded = encode_step_string(name);
    assert!(encoded.contains("\\X2\\"));
    assert_eq!(decode_step_string(&encoded).unwrap(), name);
    assert_eq!(decode_step_string("A\\X2\\03B4\\X0\\B").unwrap(), "AδB");
}

#[test]
fn semantic_export_rejects_unscoped_multi_product_metadata() {
    let model = brep_core::cuboid([0.; 3], [1., 1., 1.]).unwrap();
    let mut semantic = StepSemanticMetadata::empty(&model);
    for id in [1, 2] {
        semantic.xde.products.insert(id, StepProductMetadata {
            entity: id,
            name: format!("part {id}"),
            ..StepProductMetadata::default()
        });
    }
    let error = export_step_v9_with_metadata(&model, &semantic).unwrap_err();
    assert_eq!(error.code, "BREP_STEP_V3_REFUSED");
    assert!(error.message.contains("one XDE product"));
}

#[test]
fn product_style_is_inherited_from_assembly_parent() {
    let model = brep_core::cuboid([0.; 3], [1., 1., 1.]).unwrap();
    let mut presentation = StepPresentation::empty(&model);
    presentation.body_products[0] = Some(2);
    presentation.product_parents.insert(2, 1);
    presentation.product_styles.insert(1, colour(0.25, 0.5, 0.75));
    assert_eq!(
        presentation.face_style(0, 0).unwrap().colour,
        Some(StepColour::new(0.25, 0.5, 0.75).unwrap())
    );
}

#[test]
fn empty_metadata_has_no_hidden_product_defaults() {
    let semantic = StepSemanticMetadata::default();
    assert!(semantic.xde.products.is_empty());
    assert_eq!(semantic.presentation.product_styles, BTreeMap::new());
}
