//! Native retained-body edge editing. Fillets remain explicitly faceted.
use super::{Result, Value, brep, cad_face_selection, cad_mesh_topology, encode, field, input};
pub fn edit(v: Value) -> Result<Value> {
    let body: Value = field(&v, "body")?;
    if body.get("brep").is_none() {
        return super::cad_mesh_planes::edge(v);
    }
    let model: brep_core::Model = field(&body, "brep")?;
    let kind: String = field(&v, "kind")?;
    let size: f64 = field(&v, "size")?;
    let segments: usize = field(&v, "segments")?;
    let selected: Vec<usize> = field(&v, "edges")?;
    if selected.is_empty() {
        return Err(input("Select an edge."));
    }
    let topology = cad_mesh_topology::topology(value_codec::json!({"mesh":body["mesh"].clone()}))?;
    let displayed: Vec<Value> = field(&topology, "edges")?;
    let mut edges = Vec::new();
    for index in selected {
        let edge = displayed
            .get(index)
            .ok_or_else(|| input("Select an edge."))?;
        let a: usize = field(edge, "a")?;
        let b: usize = field(edge, "b")?;
        let id =
            cad_face_selection::edge(value_codec::json!({"body":body.clone(),"vertices":[a,b]}))?;
        let id: usize = value_codec::from_value(id).map_err(|e| input(e.to_string()))?;
        edges.push(id);
    }
    let next = match kind.as_str() {
        "chamfer" => brep_core::chamfer_edges(&model, &edges, size)?,
        "fillet" => brep_core::fillet_edges(&model, &edges, size, segments)?,
        _ => return Err(input("Invalid edge operation.")),
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
