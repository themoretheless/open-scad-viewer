//! Flat GPU encoding of supported Field trees (feature `gpu`). Translate nodes
//! fold into leaf parameters during flattening (a pure shift distributes over
//! every descendant; mesh leaves shift their triangles), so the shader needs
//! no coordinate stack. Extrude, Revolve and Deform fields stay CPU-only for
//! now.
use crate::{Field, Point};

pub const KIND_SPHERE: u32 = 0;
pub const KIND_BOX: u32 = 1;
pub const KIND_TORUS: u32 = 2;
pub const KIND_UNION: u32 = 3;
pub const KIND_INTERSECTION: u32 = 4;
pub const KIND_DIFFERENCE: u32 = 5;
pub const KIND_SMOOTH_UNION: u32 = 6;
pub const KIND_OFFSET: u32 = 7;
pub const KIND_MESH_UNSIGNED: u32 = 8;
pub const KIND_MESH_SIGNED: u32 = 9;

/// Postorder records: leaves first, value ops after their operands, so the
/// shader evaluates with a plain value stack.
pub struct FlatField {
    pub kinds: Vec<u32>,
    /// Eight f32 parameter slots per node.
    pub params: Vec<f32>,
    /// Two u32 aux slots per node (triangle window for mesh nodes).
    pub aux: Vec<u32>,
    /// Flat f32 triangle positions referenced by mesh nodes (9 floats each).
    pub triangles: Vec<f32>,
}

impl Field {
    /// Flattens the tree for the GPU interpreter; None marks node kinds the
    /// shader does not implement (the caller then uses the CPU sampler).
    pub fn to_flat(&self) -> Option<FlatField> {
        let mut flat = FlatField {
            kinds: Vec::new(),
            params: Vec::new(),
            aux: Vec::new(),
            triangles: Vec::new(),
        };
        fn leaf(flat: &mut FlatField, kind: u32, params: [f32; 8]) {
            flat.kinds.push(kind);
            flat.params.extend_from_slice(&params);
            flat.aux.extend_from_slice(&[0, 0]);
        }
        fn centered(flat: &mut FlatField, kind: u32, center: Point, offset: Point, extra: &[f32]) {
            let mut params = [0f32; 8];
            for i in 0..3 {
                params[i] = (center[i] + offset[i]) as f32;
            }
            params[3..3 + extra.len()].copy_from_slice(extra);
            leaf(flat, kind, params);
        }
        fn walk(field: &Field, offset: Point, flat: &mut FlatField) -> Option<()> {
            match field {
                Field::Sphere { center, radius } => {
                    centered(flat, KIND_SPHERE, *center, offset, &[*radius as f32])
                }
                Field::Box { center, half_size } => centered(
                    flat,
                    KIND_BOX,
                    *center,
                    offset,
                    &[
                        half_size[0] as f32,
                        half_size[1] as f32,
                        half_size[2] as f32,
                    ],
                ),
                Field::Torus {
                    center,
                    major_radius,
                    minor_radius,
                } => centered(
                    flat,
                    KIND_TORUS,
                    *center,
                    offset,
                    &[*major_radius as f32, *minor_radius as f32],
                ),
                Field::Union { a, b }
                | Field::Intersection { a, b }
                | Field::Difference { a, b } => {
                    walk(a, offset, flat)?;
                    walk(b, offset, flat)?;
                    let kind = match field {
                        Field::Union { .. } => KIND_UNION,
                        Field::Intersection { .. } => KIND_INTERSECTION,
                        _ => KIND_DIFFERENCE,
                    };
                    leaf(flat, kind, [0.; 8]);
                }
                Field::SmoothUnion { a, b, radius } => {
                    walk(a, offset, flat)?;
                    walk(b, offset, flat)?;
                    let mut params = [0.; 8];
                    params[0] = *radius as f32;
                    leaf(flat, KIND_SMOOTH_UNION, params);
                }
                Field::Offset { input, distance } => {
                    walk(input, offset, flat)?;
                    let mut params = [0.; 8];
                    params[0] = *distance as f32;
                    leaf(flat, KIND_OFFSET, params);
                }
                Field::Translate { input, vector } => {
                    walk(input, std::array::from_fn(|i| offset[i] + vector[i]), flat)?;
                }
                Field::MeshDistance { mesh, signed } => {
                    // The translate fold shifts the triangles themselves.
                    let start = (flat.triangles.len() / 9) as u32;
                    for t in mesh.indices.as_chunks::<3>().0 {
                        for &i in t {
                            for k in 0..3 {
                                flat.triangles
                                    .push((mesh.positions[3 * i + k] + offset[k]) as f32);
                            }
                        }
                    }
                    flat.kinds.push(if *signed {
                        KIND_MESH_SIGNED
                    } else {
                        KIND_MESH_UNSIGNED
                    });
                    flat.params.extend_from_slice(&[0.; 8]);
                    flat.aux
                        .extend_from_slice(&[start, (mesh.indices.len() / 3) as u32]);
                }
                Field::Extrude { .. } | Field::Revolve { .. } | Field::Deform { .. } => {
                    return None;
                }
            }
            Some(())
        }
        walk(self, [0.; 3], &mut flat)?;
        Some(flat)
    }
}
