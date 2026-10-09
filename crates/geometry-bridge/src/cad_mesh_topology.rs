//! Wire adapters for native coplanar display topology.
use super::{Result, Value, encode, field, input};
use mesh_topology::planar;
use polygon_core::Mesh;
pub fn topology(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    mesh.validate()?;
    let t = planar::topology(mesh.view()).map_err(|e| input(e.message))?;
    let faces = t.faces.into_iter().map(|f|value_codec::json!({"triangles":f.triangles,"normal":f.normal,"offset":f.offset,"vertices":f.vertices,"center":f.center})).collect::<Vec<_>>();
    let edges = t
        .edges
        .into_iter()
        .map(|e| value_codec::json!({"a":e.a,"b":e.b,"faces":e.faces}))
        .collect::<Vec<_>>();
    encode(value_codec::json!({"faces":faces,"edges":edges}))
}
pub fn face_plane(v: Value) -> Result<Value> {
    let mesh: Mesh = field(&v, "mesh")?;
    mesh.validate()?;
    let face: Value = field(&v, "face")?;
    let vertices: Vec<usize> = field(&face, "vertices")?;
    let p = planar::face_plane(mesh.view(), &vertices, field(&face, "normal")?)
        .map_err(|e| input(e.message))?;
    encode(value_codec::json!({"origin":p.origin,"u":p.u,"v":p.v}))
}
