use crate::{Result, encode, field, input};
use mesh_io::import::{self as importer, RawMesh, Weld};
use value_codec::{Value, json};
pub fn dispatch(v: Value) -> Result<Value> {
    if v["op"].as_str() == Some("stl_decode_binary") {
        let bytes: Vec<u8> = field(&v, "bytes")?;
        return encode(importer::binary_stl(&bytes)?);
    }
    if v["op"].as_str() == Some("mesh_soup_render") {
        let positions: Vec<f32> = field(&v, "positions")?;
        let mesh = mesh_topology::soup::render(&positions)?;
        return encode(
            json!({"vertices":mesh.vertices,"indices":mesh.indices,"faceIds":mesh.face_ids,"discarded":mesh.discarded}),
        );
    }

    let raw = if v["op"].as_str() == Some("mesh_import_finalize") {
        RawMesh {
            positions: field(&v, "positions")?,
            indices: if v["indices"].is_null() {
                let positions: Vec<f64> = field(&v, "positions")?;
                (0..positions.len() / 3).collect()
            } else {
                field(&v, "indices")?
            },
        }
    } else {
        let bytes: Vec<u8> = field(&v, "bytes")?;
        if bytes.len() > 20_000_000 {
            return Err(input("Mesh input exceeds byte budget"));
        }
        match v["format"].as_str() {
            Some("obj") => importer::obj(&bytes)?,
            Some("stl") => importer::stl(&bytes)?,
            Some("off") => importer::off(&bytes)?,
            Some("ply") => importer::ply(&bytes)?,
            _ => return Err(input("Unsupported native mesh format")),
        }
    };
    if v["decodeOnly"].as_bool() == Some(true) {
        return encode(json!({"positions":raw.positions,"indices":raw.indices}));
    }
    let weld = if v["weld"].as_bool() == Some(false) {
        Weld::None
    } else {
        let distance = v["weld"].as_f64().unwrap_or(0.);
        if distance > 0. {
            Weld::Grid(distance)
        } else {
            Weld::DisplayExact
        }
    };
    let imported = importer::finalize(raw, weld)?;
    encode(
        json!({"positions":imported.mesh.positions,"indices":imported.mesh.indices,"sourceVertexCount":imported.source_vertex_count,"degenerateTriangles":imported.degenerate_triangles}),
    )
}
