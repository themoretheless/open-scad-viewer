//! `mesh_svg_contours`: projects transformed triangle meshes to bounded SVG
//! contours in Rust instead of the host.
use crate::*;

struct OwnedProjectionMesh {
    vertices: Vec<f32>,
    indices: Vec<u32>,
    transform: [f64; 16],
    face_ids: Vec<u32>,
}

pub(crate) fn contours(v: &Value) -> Result<Value> {
    let raw_meshes = v["meshes"].as_array().ok_or_else(|| {
        input("SVG projection requires a valid triangle mesh and affine transform.")
    })?;
    let mut owned = Vec::with_capacity(raw_meshes.len());
    for item in raw_meshes {
        owned.push(OwnedProjectionMesh {
            vertices: field::<Vec<u32>>(item, "vertexBits")?
                .into_iter()
                .map(f32::from_bits)
                .collect(),
            indices: field::<Vec<u32>>(item, "indices")?,
            transform: field::<[f64; 16]>(item, "transform")?,
            face_ids: field::<Vec<u32>>(item, "faceIds")?,
        });
    }
    let views: Vec<mesh_section::SvgProjectionMesh<'_>> = owned
        .iter()
        .map(|m| mesh_section::SvgProjectionMesh {
            vertices: &m.vertices,
            indices: &m.indices,
            transform: m.transform,
            face_ids: &m.face_ids,
        })
        .collect();
    let axis = optional_field::<String>(v, "axis")?;
    let face_triangle = optional_field::<usize>(v, "faceTriangle")?;
    encode(mesh_section::svg_contours(
        &views,
        axis.as_deref(),
        face_triangle,
    )?)
}
