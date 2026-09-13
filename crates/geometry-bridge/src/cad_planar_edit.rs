//! Transactional retained-body planar edits using native display selection.
use super::{Result, Value, brep, cad_face_selection, cad_mesh_topology, encode, field, input};
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
    let topology = cad_mesh_topology::topology(value_codec::json!({"mesh":body["mesh"].clone()}))?;
    let displayed: Vec<Value> = field(&topology, "faces")?;
    let mut faces = Vec::new();
    for index in selected {
        let face = displayed
            .get(index)
            .ok_or_else(|| input("Select a face."))?;
        let id = cad_face_selection::select(
            value_codec::json!({"body":body.clone(),"triangles":face["triangles"].clone()}),
        )?;
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
