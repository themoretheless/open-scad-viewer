//! STEP B-rep import with frontend-ready product, body and face display data.
//! The parent owns module and route registration.

use super::{Result, Value, field, input};
use brep_core::Model;
use cad_step::step_interchange_v3::step_xde::StepValidationKind;
use cad_step::step_interchange_v3::{
    StepColour, StepPresentationStyle, StepProductMetadata, import_step_v9_with_metadata,
};
use value_codec::json;

fn channel(value: f64) -> Result<u8> {
    if !value.is_finite() {
        return Err(input("STEP display colour channels must be finite"));
    }
    Ok((value.clamp(0., 1.) * 255.).round() as u8)
}

fn rgba(colour: Option<StepColour>) -> Result<Value> {
    colour
        .map(|colour| {
            Ok(json!([
                channel(colour.red)?,
                channel(colour.green)?,
                channel(colour.blue)?,
                255
            ]))
        })
        .transpose()
        .map(|rgba| rgba.unwrap_or(Value::Null))
}

fn display(style: Option<StepPresentationStyle>) -> Result<Value> {
    let (colour, layers) = style
        .map(|style| (style.colour, style.layers))
        .unwrap_or((None, Vec::new()));
    Ok(json!({"rgba":rgba(colour)?,"layers":layers}))
}

fn body_face_indices(model: &Model, body_index: usize) -> Vec<usize> {
    let body = &model.bodies[body_index];
    std::iter::once(body.outer_shell)
        .chain(body.inner_shells.iter().copied())
        .flat_map(|shell| model.shells[shell].faces.iter().map(|usage| usage.face))
        .collect()
}

fn product_json(
    product: &StepProductMetadata,
    style: Option<StepPresentationStyle>,
    bodies: Vec<Value>,
    diagnostics: Vec<Value>,
) -> Result<Value> {
    Ok(json!({
        "entity":product.entity,
        "id":product.id,
        "name":product.name,
        "description":product.description,
        "shapeName":product.shape_name,
        "appliedNames":product.applied_names,
        "material":product.material,
        "densityKgPerM3":product.density_kg_per_m3,
        "validation":{
            "volumeMm3":product.validation.volume_mm3,
            "surfaceAreaMm2":product.validation.surface_area_mm2,
        },
        "validationDiagnostics":diagnostics,
        "display":display(style)?,
        "bodies":bodies,
    }))
}

/// Import `{text, segments?}` as an exact B-rep plus scene/display JSON.
/// A mesh is tessellated only when `segments` is explicitly present.
pub(crate) fn import(v: Value) -> Result<Value> {
    let object = v.as_object().ok_or_else(|| {
        input("STEP scene import expects an object containing text and optional segments")
    })?;
    if !object.contains_key("text")
        || object
            .keys()
            .any(|key| !matches!(key.as_str(), "op" | "text" | "segments"))
    {
        return Err(input(
            "STEP scene import expects op, text and optional segments only",
        ));
    }
    let text: String = field(&v, "text")?;
    let segments = object
        .contains_key("segments")
        .then(|| field::<usize>(&v, "segments"))
        .transpose()?;
    if let Some(segments) = segments
        && !(1..=32).contains(&segments)
    {
        return Err(input("STEP scene tessellation segments must be 1..32"));
    }

    let (model, certificate, report, semantic) = import_step_v9_with_metadata(&text)?;
    let diagnostics = if semantic.xde.products.len() == 1 {
        semantic.xde.validation_diagnostics(&model, 1e-7, 1e-6)?
    } else {
        // An aggregate assembly mass cannot validate a component's authored value.
        Vec::new()
    };
    let diagnostics: Vec<Value> = diagnostics
        .into_iter()
        .map(|diagnostic| {
            json!({
                "code":diagnostic.code,
                "product":diagnostic.product,
                "property":match diagnostic.property {
                    StepValidationKind::Volume => "volume",
                    StepValidationKind::SurfaceArea => "surfaceArea",
                },
                "declared":diagnostic.declared,
                "computed":diagnostic.computed,
                "absoluteError":diagnostic.absolute_error,
                "relativeError":diagnostic.relative_error,
                "tolerance":diagnostic.tolerance,
            })
        })
        .collect();

    let mut body_nodes = Vec::with_capacity(model.bodies.len());
    let mut flat_bodies = Vec::with_capacity(model.bodies.len());
    let mut flat_faces = Vec::new();
    for body_index in 0..model.bodies.len() {
        let product_entity = semantic.xde.body_products.get(body_index).copied().flatten();
        let body_style = semantic.presentation.body_style(body_index);
        let body_display = display(body_style)?;
        let mut faces = Vec::new();
        for face_index in body_face_indices(&model, body_index) {
            let face_display = display(semantic.presentation.face_style(face_index, body_index))?;
            let topology_face_id = model.1.faces[face_index].to_string();
            flat_faces.push(json!({
                "faceIndex":face_index,
                "topologyFaceId":topology_face_id,
                "bodyIndex":body_index,
                "productEntity":product_entity,
                "rgba":face_display["rgba"],
                "layers":face_display["layers"],
            }));
            faces.push(json!({
                "faceIndex":face_index,
                "topologyFaceId":topology_face_id,
                "display":face_display,
            }));
        }
        let topology_body_id = model.1.bodies[body_index].to_string();
        flat_bodies.push(json!({
            "bodyIndex":body_index,
            "topologyBodyId":topology_body_id,
            "productEntity":product_entity,
            "rgba":body_display["rgba"],
            "layers":body_display["layers"],
        }));
        body_nodes.push(json!({
            "bodyIndex":body_index,
            "topologyBodyId":topology_body_id,
            "display":body_display,
            "faces":faces,
        }));
    }

    let mut products = Vec::with_capacity(semantic.xde.products.len());
    let mut flat_products = Vec::with_capacity(semantic.xde.products.len());
    for (&entity, product) in &semantic.xde.products {
        let product_display = display(semantic.presentation.product_styles.get(&entity).cloned())?;
        flat_products.push(json!({
            "productEntity":entity,
            "rgba":product_display["rgba"],
            "layers":product_display["layers"],
        }));
        let bodies = semantic
            .xde
            .body_products
            .iter()
            .enumerate()
            .filter(|(_, owner)| **owner == Some(entity))
            .map(|(body_index, _)| body_nodes[body_index].clone())
            .collect();
        let product_diagnostics = diagnostics
            .iter()
            .filter(|diagnostic| diagnostic["product"] == json!(entity))
            .cloned()
            .collect();
        products.push(product_json(
            product,
            semantic.presentation.product_styles.get(&entity).cloned(),
            bodies,
            product_diagnostics,
        )?);
    }

    let unsupported_presentation: Vec<Value> = semantic
        .presentation
        .unsupported
        .iter()
        .map(|item| json!({"code":item.code,"entity":item.entity,"detail":item.detail}))
        .collect();
    let unsupported_xde: Vec<Value> = semantic
        .xde
        .unsupported
        .iter()
        .map(|item| json!({"code":item.code,"entity":item.entity,"detail":item.detail}))
        .collect();
    let mut result = json!({
        "model":model,
        "certificate":{
            "capability":certificate.capability,
            "complete":certificate.complete,
            "notes":certificate.notes,
        },
        "report":{
            "identity":{
                "preserved":report.identity.preserved,
                "source":report.identity.source,
                "preservedCount":report.identity.preserved_count,
                "createdCount":report.identity.created_count,
                "lostCount":report.identity.lost_count,
            },
            "ignoredEntities":report.ignored_entities,
            "metadataLoss":report.metadata_loss,
            "instanceCount":report.instance_count,
            "reachableCount":report.reachable_count,
        },
        "sceneTree":{
            "kind":"stepScene",
            "products":products,
            "unsupported":{
                "presentation":unsupported_presentation,
                "xde":unsupported_xde,
            },
        },
        "display":{
            "products":flat_products,
            "bodies":flat_bodies,
            "faces":flat_faces,
        },
    });
    if let Some(segments) = segments {
        result["mesh"] = json!(super::brep::nurbs(&model, segments)?);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cad_step::step_interchange_v3::step_xde::StepValidationProperties;
    use cad_step::step_interchange_v3::{
        StepProductMetadata, StepSemanticMetadata, export_step_v9_with_metadata,
    };

    fn style(red: f64, green: f64, blue: f64, layer: &str) -> StepPresentationStyle {
        StepPresentationStyle {
            colour: Some(StepColour::new(red, green, blue).unwrap()),
            layers: vec![layer.into()],
        }
    }

    fn semantic_fixture() -> String {
        let model = brep_core::cuboid([0., 0., 0.], [2., 3., 4.]).unwrap();
        let mut semantic = StepSemanticMetadata::empty(&model);
        semantic.xde.products.insert(42, StepProductMetadata {
            entity: 42,
            id: "part-42".into(),
            name: "Корпус δ".into(),
            description: "frontend scene fixture".into(),
            shape_name: Some("Основная форма".into()),
            applied_names: vec!["Деталь 42".into()],
            material: Some("Сталь 09Г2С".into()),
            density_kg_per_m3: Some(7850.),
            validation: StepValidationProperties {
                volume_mm3: Some(24.),
                surface_area_mm2: Some(52.),
            },
        });
        semantic.xde.body_products[0] = Some(42);
        semantic.presentation.body_products[0] = Some(42);
        semantic.presentation.product_styles.insert(42, style(0., 0., 1., "Product layer"));
        semantic.presentation.body_styles[0] = Some(style(0., 1., 0., "Body layer"));
        semantic.presentation.face_styles[0] = Some(style(1., 0., 0., "Face layer"));
        export_step_v9_with_metadata(&model, &semantic).unwrap().0
    }

    #[test]
    fn scene_display_and_optional_mesh_follow_imported_topology() {
        let text = semantic_fixture();
        let plain = import(json!({"text":text})).unwrap();
        assert_ne!(plain["model"], Value::Null);
        assert_eq!(plain["mesh"], Value::Null);
        let product = &plain["sceneTree"]["products"][0];
        assert_eq!(product["name"], json!("Корпус δ"));
        assert_eq!(product["material"], json!("Сталь 09Г2С"));
        assert_eq!(product["densityKgPerM3"], json!(7850.));
        assert_eq!(product["validationDiagnostics"], json!([]));
        assert_eq!(product["display"]["rgba"], json!([0, 0, 255, 255]));
        let faces = &product["bodies"][0]["faces"];
        assert_eq!(faces[0]["display"]["rgba"], json!([255, 0, 0, 255]));
        assert_eq!(faces[0]["display"]["layers"], json!(["Face layer"]));
        assert_eq!(faces[1]["display"]["rgba"], json!([0, 255, 0, 255]));
        assert_eq!(faces[0]["faceIndex"], json!(0));
        assert_eq!(faces[0]["topologyFaceId"], plain["display"]["faces"][0]["topologyFaceId"]);
        assert_eq!(plain["display"]["products"][0]["productEntity"], product["entity"]);
        assert_eq!(plain["display"]["bodies"][0]["bodyIndex"], json!(0));

        let meshed = import(json!({"text":semantic_fixture(),"segments":2})).unwrap();
        assert!(meshed["mesh"]["faceIds"].is_array());
        assert!(meshed["mesh"]["topologyFaceIds"].is_array());
    }

    #[test]
    fn invalid_segments_are_typed_input_errors() {
        for segments in [json!(0), json!(33), json!("2"), json!(f64::NAN)] {
            let error = import(json!({"text":"not parsed","segments":segments})).unwrap_err();
            assert_eq!(error.code, "GEOMETRY_INVALID_INPUT");
        }
    }

    #[test]
    fn public_dispatch_reaches_semantic_scene_import() {
        let output = crate::dispatch(json!({
            "op":"brep_nurbs_import_step_scene",
            "text":semantic_fixture()
        }))
        .unwrap();
        assert_eq!(output["sceneTree"]["kind"], json!("stepScene"));
        assert_eq!(output["display"]["faces"][0]["rgba"], json!([255, 0, 0, 255]));
    }
}
