//! Transactional retained-body planar edits using native display selection.
use super::{Result, Value, brep, cad_face_selection, encode, field, input};
pub fn edit(v: Value) -> Result<Value> {
    let body: Value = field(&v, "body")?;
    if body.get("brep").is_none() {
        return super::cad_mesh_planes::planar(v);
    }
    let model: brep_core::Model = field(&body, "brep")?;
    let action: String = field(&v, "action")?;
    let amount: f64 = field(&v, "amount")?;
    let selected: Vec<usize> = field(&v, "faces")?;
    if selected.is_empty() {
        return Err(input("Select a face."));
    }
    if !amount.is_finite() {
        return Err(input("Enter a finite distance."));
    }
    let mesh: polygon_core::Mesh = field(&body, "mesh")?;
    mesh.validate()?;
    let displayed = mesh_topology::planar::topology(mesh.view())
        .map_err(|e| input(e.message))?
        .faces;
    let mut faces = Vec::new();
    for index in selected {
        let face = displayed
            .get(index)
            .ok_or_else(|| input("Select a face."))?;
        let select = if action == "push" {
            cad_face_selection::select_cap
        } else {
            cad_face_selection::select
        };
        let id = select(value_codec::json!({"body":body.clone(),"triangles":face.triangles}))?;
        let id: usize = value_codec::from_value(id).map_err(|e| input(e.to_string()))?;
        if !faces.contains(&id) {
            faces.push(id);
        }
    }
    let next = match action.as_str() {
        "push" => {
            if faces.len() != 1 {
                return Err(input("Select exactly one face."));
            }
            brep_core::operations::push_planar_face(&model, faces[0], amount)?
        }
        "shell" => {
            if amount < 0.01 {
                return Err(input("Wall thickness must be at least 0.01 mm."));
            }
            if faces.len() >= model.faces.len() {
                return Err(input("Keep at least one closed face."));
            }
            brep_core::operations::shell_planar(&model, &faces, amount)?
        }
        _ => return Err(input("Invalid planar editing action.")),
    };
    let mesh = brep::nurbs(&next, 1)?.built.mesh;
    let mut result = body
        .as_object()
        .ok_or_else(|| input("Expected body record"))?
        .clone();
    result.insert("brep".into(), encode(next)?);
    result.insert("mesh".into(), encode(mesh)?);
    Ok(Value::Object(result))
}

#[cfg(test)]
mod tests {
    use super::super::cad_mesh_topology;
    use super::*;
    #[test]
    fn cap_selection_is_scoped_and_preserves_input_on_success_and_failure() {
        let fixture: Value = value_codec::from_str(include_str!(
            "../../../docs/qualification/cad-roadmap-2026-09-28/parts-history/cap-api-fixtures.json"
        ))
        .unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let request = case["request"].clone();
            let before = request.to_string();
            let result = edit(request.clone()).unwrap();
            let model: brep_core::Model = field(&result, "brep").unwrap();
            let mass = brep_core::analysis::mass_properties(&model, 1e-7, 200_000)
                .unwrap()
                .signed_volume_mm3;
            assert!((mass - case["expectedVolume"].as_f64().unwrap()).abs() < 1e-5);
            assert_eq!(result["id"], request["body"]["id"]);
            let source: brep_core::Model = field(&request["body"], "brep").unwrap();
            let mut fixed = 0;
            for (i, v) in source.vertices.iter().enumerate() {
                if let Some(j) = model.vertices.iter().position(|p| p.point == v.point) {
                    assert_eq!(source.1.vertices[i], model.1.vertices[j]);
                    fixed += 1;
                }
            }
            assert!(fixed > 0);

            assert_eq!(before, request.to_string());
            let mut consumed = request.clone();
            consumed["amount"] = value_codec::json!(-20.);
            assert!(edit(consumed).is_err());
            if case["name"].as_str() == Some("enclosure") {
                let topology = cad_mesh_topology::topology(
                    value_codec::json!({"mesh":request["body"]["mesh"]}),
                )
                .unwrap();
                let index = request["faces"][0].as_u64().unwrap() as usize;
                let selection = value_codec::json!({"body":request["body"],"triangles":topology["faces"][index]["triangles"]});
                assert!(cad_face_selection::select(selection.clone()).is_err());
                assert!(cad_face_selection::select_cap(selection).is_ok());
                let mut shell = request.clone();
                shell["action"] = value_codec::json!("shell");
                assert!(edit(shell).is_err());
            }
            assert_eq!(before, request.to_string());
        }
    }
}
