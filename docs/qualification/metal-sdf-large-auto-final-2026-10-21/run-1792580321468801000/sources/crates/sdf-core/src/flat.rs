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
pub const KIND_CAPSULE: u32 = 10;
pub const KIND_SMOOTH_DIFFERENCE: u32 = 11;
pub const KIND_CYLINDER: u32 = 12;
pub const KIND_GYROID: u32 = 13;
pub const KIND_SCHWARZ_P: u32 = 14;
pub const KIND_DIAMOND: u32 = 15;

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
        fn count_nodes(field: &Field) -> (usize, usize) {
            match field {
                Field::Union { a, b }
                | Field::Intersection { a, b }
                | Field::Difference { a, b }
                | Field::SmoothUnion { a, b, .. }
                | Field::SmoothDifference { a, b, .. } => {
                    let (na, ta) = count_nodes(a);
                    let (nb, tb) = count_nodes(b);
                    (na + nb + 1, ta + tb)
                }
                Field::Offset { input, .. } => {
                    let (n, t) = count_nodes(input);
                    (n + 1, t)
                }
                Field::Translate { input, .. } => count_nodes(input),
                Field::MeshDistance { mesh, .. } => (1, mesh.indices.len() * 3),
                _ => (1, 0),
            }
        }
        let (node_cap, tri_cap) = count_nodes(self);
        let mut flat = FlatField {
            kinds: Vec::with_capacity(node_cap),
            params: Vec::with_capacity(node_cap * 8),
            aux: Vec::with_capacity(node_cap * 2),
            triangles: Vec::with_capacity(tri_cap),
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
                Field::Cylinder {
                    center,
                    radius,
                    half_height,
                } => centered(
                    flat,
                    KIND_CYLINDER,
                    *center,
                    offset,
                    &[*radius as f32, *half_height as f32],
                ),
                Field::Gyroid {
                    origin,
                    period,
                    thickness,
                } => centered(
                    flat,
                    KIND_GYROID,
                    *origin,
                    offset,
                    &[*period as f32, *thickness as f32],
                ),
                Field::SchwarzP {
                    origin,
                    period,
                    thickness,
                } => centered(
                    flat,
                    KIND_SCHWARZ_P,
                    *origin,
                    offset,
                    &[*period as f32, *thickness as f32],
                ),
                Field::Diamond {
                    origin,
                    period,
                    thickness,
                } => centered(
                    flat,
                    KIND_DIAMOND,
                    *origin,
                    offset,
                    &[*period as f32, *thickness as f32],
                ),
                Field::Capsule { a, b, radius } => {
                    let mut params = [0f32; 8];
                    for i in 0..3 {
                        params[i] = (a[i] + offset[i]) as f32;
                        params[3 + i] = (b[i] + offset[i]) as f32;
                    }
                    params[6] = *radius as f32;
                    leaf(flat, KIND_CAPSULE, params);
                }
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
                Field::SmoothUnion { a, b, radius } | Field::SmoothDifference { a, b, radius } => {
                    walk(a, offset, flat)?;
                    walk(b, offset, flat)?;
                    let mut params = [0.; 8];
                    params[0] = *radius as f32;
                    let kind = match field {
                        Field::SmoothUnion { .. } => KIND_SMOOTH_UNION,
                        _ => KIND_SMOOTH_DIFFERENCE,
                    };
                    leaf(flat, kind, params);
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
                    for &[a, b, c] in mesh.indices.as_chunks::<3>().0 {
                        let [ax, ay, az] = [
                            (mesh.positions[3 * a] + offset[0]) as f32,
                            (mesh.positions[3 * a + 1] + offset[1]) as f32,
                            (mesh.positions[3 * a + 2] + offset[2]) as f32,
                        ];
                        let [bx, by, bz] = [
                            (mesh.positions[3 * b] + offset[0]) as f32,
                            (mesh.positions[3 * b + 1] + offset[1]) as f32,
                            (mesh.positions[3 * b + 2] + offset[2]) as f32,
                        ];
                        let [cx, cy, cz] = [
                            (mesh.positions[3 * c] + offset[0]) as f32,
                            (mesh.positions[3 * c + 1] + offset[1]) as f32,
                            (mesh.positions[3 * c + 2] + offset[2]) as f32,
                        ];
                        flat.triangles
                            .extend_from_slice(&[ax, ay, az, bx, by, bz, cx, cy, cz]);
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
                Field::Extrude { .. }
                | Field::Revolve { .. }
                | Field::Deform { .. }
                | Field::Neovius { .. }
                | Field::Lidinoid { .. }
                | Field::SchoenIWP { .. }
                | Field::Octahedron { .. }
                | Field::HexPrism { .. }
                | Field::TriangularPrism { .. }
                | Field::SmoothIntersection { .. }
                | Field::ChamferUnion { .. }
                | Field::ChamferIntersection { .. }
                | Field::ChamferDifference { .. }
                | Field::Onion { .. }
                | Field::Elongate { .. }
                | Field::Mirror { .. }
                | Field::Taper { .. } => {
                    return None;
                }
            }
            Some(())
        }
        walk(self, [0.; 3], &mut flat)?;
        Some(flat)
    }
}
