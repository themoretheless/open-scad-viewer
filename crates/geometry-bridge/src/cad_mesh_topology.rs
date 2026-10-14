//! Wire adapters for native coplanar display topology.
use super::{Result, Value, encode, field, input};
use math_core::dot;
use polygon_core::Mesh;
use std::collections::{BTreeMap, BTreeSet, HashMap};
struct Face {
    triangles: Vec<usize>,
    normal: [f64; 3],
    offset: f64,
    vertices: Vec<usize>,
    center: [f64; 3],
}
struct Edge {
    a: usize,
    b: usize,
    faces: Vec<usize>,
}
// Fixed-seven decimal rounding, with JavaScript's ties-away rule for the
// legacy display seam key. Multiplying in binary64 first would lose ties.
fn seam_coordinate(x: f64) -> String {
    if x.abs() >= 1e21 {
        return format!("{x:.7}");
    }
    let bits = x.abs().to_bits();
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let mantissa = (bits & ((1u64 << 52) - 1)) | if exponent == 0 { 0 } else { 1u64 << 52 };
    let power = if exponent == 0 {
        -1074
    } else {
        exponent - 1023 - 52
    };
    let numerator = mantissa as u128 * 10_000_000;
    let rounded = if power >= 0 {
        numerator << power
    } else {
        let shift = (-power) as u32;
        if shift >= 128 {
            0
        } else {
            (numerator + (1u128 << (shift - 1))) >> shift
        }
    };
    format!(
        "{}{}.{:07}",
        if x < 0. { "-" } else { "" },
        rounded / 10_000_000,
        rounded % 10_000_000
    )
}
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

use mesh_topology::planar;
